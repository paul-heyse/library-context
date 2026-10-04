# M4 inventory: orchestration, PostgreSQL effects, CLI, embedding client and Python/PyO3 bridge

Supporting evidence for the [library-leverage review](../../reviews/design_review_library-leverage_2026-10-04.md). Repository-fact inventory returned by the M4 code-mapper lane (read-only, baseline `948b2a88`); published verbatim by the coordinator. No verdicts.

Baseline `948b2a88`. `git log 948b2a88..HEAD` shows no commits touching `crates/cpg-core/src`, `crates/lctx-postgres/src`, `crates/lctx/src`, `crates/lctx-embed/src` or `python/`, so all citations match HEAD. Working-tree changes at the time are only `docs/library-utilization.jsonl` and the untracked evidence folder. Everything below was read as source on 2026-10-04; nothing was built or run. Line ranges are approximate.

Facts that correct the brief's leads:

- All 46 `AssertSqlSafe` uses in `src/` are in `lctx-postgres`. `cpg-core/src` has zero. Another 288 are in tests (cpg-core 199, lctx-postgres 59, lctx 30).
- The `format!` SQL in `cpg-core/src` never touches PostgreSQL. Its 19 `SELECT * FROM "{NAME}"` strings are planned by DataFusion through `crate::sql::query` (`cpg-core/src/sql.rs`), with one exception, `generation_read.rs:783`, which builds text for PostgreSQL.
- `sqlx::query!` and the `.sqlx` offline metadata are unused (0 uses; `.sqlx` has only `.` and `..`). `docs/postgresql.md:167` records this deliberately.
- sea-query is used only in `ddl.rs`, for `Table::create`, `ForeignKey::create` and `Index` inside the table builder. Every `CHECK` is `Expr::cust(format!…)` (10 uses). Views, indexes, schemas, grants, revokes and delta tables are `format!` text (58 `format!` calls in `ddl.rs`).
- ADR-0086:40 says "SeaQuery lowers validated descriptors". ADR-0068 and ADR-0069 are not in `docs/adr/`.

## 1. Candidate inventory

**M4-01 · Generation DDL lowering**
- Location: `lctx-postgres/src/generations/ddl.rs:213-412` (`lower` tables), `:412-540` (views, serving mappings, indexes, grants), `:545-665` (derivation views, prefix views, delta tables, introduction FKs), `:155-212` (`control()` templating `control.sql`, 164 lines, via `.replace("{token}")`).
- Capability: lower a validated relational model into ordered DDL phases (staging, sealed, validated, published), with checks, FKs, views, grants and a physical digest.
- Consumers: install, create-generation, seal/validate/publish, `verify::check`, the lease contract, `serving_shape`.
- Invariants: pure, deterministic lowering (one model, generation, schema and control name always give the same statements), and the digest of the DDL is the identity checked by leases. The header comment states this is so a shadow install is a faithful reference.
- Library inside: sea-query (`Table`, `ColumnDef`, `ColumnType`, `ForeignKey`, `Index`, `PostgresQueryBuilder`).
- Stated reason: `ddl.rs:1-3` and ADR-0086:40. No reason is given for leaving views, grants and indexes outside sea-query. `storage-and-publication.md:137` lists SeaQuery as "later variable joins/expressions".
- Repetition: the identity-row view text `SELECT cols FROM schema.source` is built in `ddl.rs:440-460` and rebuilt in `serving_shape.rs:20-50`, which compares it to `pg_get_viewdef` after stripping whitespace and quotes. `column_signature` (`ddl.rs:67-103`) restates the Scalar→PG type map from `lower` (`:238-248`) a second time, and `codec.rs` maps Arrow type→Rust type a third time.
- Tests: `lctx-postgres/tests/installation.rs`, `generations.rs`, `serving_shapes.rs`, `domain_derivation.rs`.
- Library would have to provide (observation): typed DDL for `CREATE VIEW`, `CREATE INDEX`, `GRANT`/`REVOKE`, `CREATE SCHEMA` and `CTAS WITH NO DATA`, plus identifier and literal escaping. sea-query 1.0.2 has these builders on its public surface, but whether they are enabled under `default-features=false` plus `backend-postgres`/`postgres-array` was not checked.

**M4-02 · Identifier quoting and literal escaping**
- Location: `generations/mod.rs:1172` (`quoted`), `:1175` (`qualified`), `cpg-core/src/generation_read.rs:614` (`quote`, an identical copy), `ddl.rs:549` (`literal`), plus ad-hoc `"\"{}\""` in `ddl.rs:~277` and `stage_validation.rs:~88,~230`.
- Capability: safe SQL identifier and literal rendering.
- Consumers: every `format!` SQL site in §2.
- Invariants: identifiers come only from the validated model, generation ids (32 hex) or fixed constants. Values are bound, except generation hex, epoch and tag/code integers, which are inlined (`ddl.rs:~215-224`, `vocabulary.rs:266`).
- Library inside: none.
- Stated reason: `docs/postgresql.md:167` says serving "binds values and derives relation identifiers from the validated model". No reason is given for the bespoke quoting.
- Repetition: two copies of `quote(d)`. Raw `"{name}"` without escaping appears in `stage_validation.rs` (`a.\"{}\"`), `cpg-core/model_runtime.rs:~139` and `catalog_*` (`SELECT * FROM "{NAME}"`).
- Tests: `cpg-core/tests/sql_boundary.rs`, `lctx-postgres/tests/vocabulary_epochs.rs`.
- Would have to provide: `Iden`/`Quote` style escaping, or DataFusion's unparser identifier quoting.

**M4-03 · Nominal-reference EXISTS/NOT EXISTS check SQL**
- Location: `vocabulary.rs:299-331`, `stage_validation.rs:~85-100` and `:~215-255`. The same text also exists as FK constraints from `ddl.rs:330-375` (sea-query `ForeignKey`).
- Capability: "every non-null reference column resolves to a row in the target, optionally with a subtype tag, in a prefix-scoped physical relation".
- Consumers: checkpoint, vocabulary close and stage-input validation.
- Invariants: prefix (epoch) scoping, so a later row never grants visibility. Failure is `Error::Contract`.
- Library inside: none (text).
- Stated reason: comments at `vocabulary.rs:~290` ("Merely existing in a future base row never establishes visibility").
- Repetition: three `format!` copies plus the FK declarations, which are installed transiently under `SAVEPOINT … ROLLBACK TO` (`vocabulary.rs:~280`, `receipts.rs:~375`) to reuse FK validation.
- Tests: `stage_validation.rs`, `vocabulary_epochs.rs`, `publication_checks.rs`.
- Would have to provide: a typed anti-join/`EXISTS` builder, or `NOT VALID`/`VALIDATE CONSTRAINT`.

**M4-04 · Live-schema vs shadow-install diff (`store check`)**
- Location: `verify.rs:1-650`: `DESCRIBE` catalog query (`:561-596`, 11 UNION ALL branches over `pg_*`), `describe` (`:598`), `compare_objects` (`:618`), `roles` (`:~385`), `database` (`:~455`), `inventory` (`:~490`).
- Capability: structural diff of an installed PostgreSQL schema against the expected schema, with findings of kind Missing, Unexpected, Differs, Orphan, Role, Database or Installation.
- Consumers: `lctx store check`, `GenerationStore::check`.
- Invariants: runs under an exclusive installation lock taken by trying; shadows are created inside a rolled-back transaction under savepoints; names are normalized with placeholder substitution (`describe` `replacements`).
- Library inside: SQLx only. The diff, text normalization and finding taxonomy are bespoke.
- Stated reason: module header `verify.rs:1-10` explains that `pg_get_*def` reads the latest catalog, so a REPEATABLE READ snapshot would not suffice. It does not say why no diff tool is used.
- Repetition: a second, narrower hand-written catalog inspector for `lctx_cache.serving_vector*` in `vectors.rs:385-670` (`inspect_cache`, roughly 285 lines: `pg_class`, `pg_attribute`, `pg_constraint`, `pg_index` and `aclexplode` queries, all with `LIMIT n`). Also `ddl::LIVE_COLUMNS` (`ddl.rs:106`), a third column-signature mechanism run on every lease.
- Tests: `tests/installation.rs`, `tests/serving_shapes.rs`, `lctx/tests/store_cli.rs`.
- Would have to provide: schema-diff or catalog introspection that can run inside a transaction against a shadow schema, with ACLs, constraints and view definitions.

**M4-05 · Binary COPY with per-row byte accounting**
- Location: `generations/mod.rs:575-703` (`copy_into`), `:705-722` (`copy_size`).
- Capability: stream an Arrow `RecordBatch` into PostgreSQL by binary COPY, with a memory budget, a row-size cap and abort on refusal.
- Consumers: `GenerationAttempt::copy` (`lifecycle.rs:369`, `:802`).
- Invariants: no row over `MAX_ROW_BYTES` (minus 20 or 26 wire bytes of metadata), every wire buffer admitted to `ResourceBudget` before allocation, COPY abort is confirmed or reported as `CopyAbort`, generation and attempt locks held.
- Library inside: pgpq (`ArrowToPostgresBinaryEncoder`, `EncoderBuilder`, `byte_size_hint`) and sqlx `copy_in_raw`.
- Stated reason: ADR-0086:40 ("pgpq owns binary COPY"). The row-at-a-time `slice(index,1)` plus size hint is the bespoke part; the code comment ties it to budget admission.
- Repetition: none elsewhere. The cache insert uses `QueryBuilder::push_values` (`cache.rs:156`).
- Tests: `tests/generations.rs` (oversized row), `generation_stages.rs`.
- Would have to provide: bounded-memory COPY with pre-admission of row sizes.

**M4-06 · Result decoding and type dispatch**
- Location: `generations/codec.rs:1-88` (`decode(relation, &[PgRow])`), `mod.rs:1041-1130` (`visit_physical_inner`: manual row-byte accounting via `try_get_raw` and chunking at `TRANSFER_ROWS`/`TRANSFER_BYTES`).
- Capability: PgRow to declared Arrow arrays, with a size-bounded streaming visitor.
- Invariants: exact declared Arrow schema, list element type only up to Int64/Utf8/Bool/Int16/Int32, `ORDER BY … COLLATE "C"` on text, `SELECT DISTINCT` on `__delta_` tables, row-size cap, memory held as `3×` bytes.
- Library inside: sqlx `Row`/`try_get`, arrow-array builders.
- Stated reason: `codec.rs:1` ("Type dispatch lives only at this effect boundary").
- Repetition: the other read path, `generation_read.rs`, goes through the forked `datafusion-table-providers-postgres` (`query_arrow_bounded`). So the repo has two PostgreSQL→Arrow readers: the SQLx path (owner/lifecycle, receipts, validation) and the provider path (stage inputs, inspection).
- Tests: `tests/generations.rs`, `unicode_values.rs`.
- Would have to provide: row-to-Arrow with explicit schema and byte accounting. The forked provider already offers `ChunkLimits` and `query_arrow_bounded`.

**M4-07 · Generation read provider (DataFusion table providers over leased connections)**
- Location: `cpg-core/src/generation_read.rs:1-920`, namely `GenerationTable` (`:738-820`), `GenerationScan` (`:822-920`), `admits_expr`, `postgres_expr`, `unqualified`, `view_cast` (`:646-737`), `InspectionSession` (`:~395-470`), `OutsideFrontier`, `Binder`/`Tokio` (`:103-200`).
- Capability: a closed-predicate pushdown `TableProvider` over one pinned generation relation, with per-connection leases, a lost-session terminal state and read-only SQL inspection.
- Invariants: lease taken on every bound connection, never reconnects; filters admitted only if in a closed algebra (text compared only for equality); `Exact` pushdown only for admitted expressions; frontier-outside relations raise a typed error.
- Libraries inside: DataFusion `TableProvider`, `Unparser` with `PostgreSqlDialect` (`:~780`), `SQLOptions::verify_plan`, `MemoryConsumer`, `RecordBatchStreamAdapter`, and the forked `datafusion-table-providers-postgres` pool (`new_bound`, `SessionBinder`, `BoundLimits`, `ChunkLimits`).
- Stated reason: module header `:1-8`.
- Repetition: `SELECT {cols} FROM {src} WHERE … LIMIT n` is built by `format!` at `:783-790`, with column and source quoting from the second copy of `quote`. Only the predicate goes through the unparser. `classify()` (`:103-117`) repeats `failure.rs:sqlstate`, and the detail string `SQLSTATE x constraint y table z` repeats `failure.rs:Failure::of` (`:~145-155`).
- Tests: `cpg-core/tests/generation_read.rs`, `sql_boundary.rs`.
- Would have to provide: a plan-to-SQL unparse of the whole scan (projection, filter, limit) rather than the predicate only.

**M4-08 · Prepared-query admission and attempt runtime (DataFusion wrapper)**
- Location: `cpg-core/src/model_runtime.rs:1-368` (`AttemptRuntime`, `StageSession`, `ComputePool` adapter to `ResourceBudget`), `model_runtime/prepared.rs:1-217` (`PreparedQuery`, `count_scans`, `QueryStream`).
- Capability: capability-gated table registration, physical-plan walking to count remote scans against pool capacity, a single-flight gate (`try_lock_owned`), and a memory pool with peak recording.
- Libraries inside: DataFusion `RuntimeEnvBuilder`, `PeakRecordingPool`, `SessionStateBuilder`, `MemTable`, `execute_stream`; `tokio::sync::Mutex`.
- Stated reason: comments in `prepared.rs` ("No DataFrame or ExecutionPlan escapes this wrapper, so execution cannot bypass admission").
- Repetition: `ComputePool`/`ComputeReservation` adapt DataFusion's memory pool to `lctx_model::domain::resources::ResourceBudget`. A parallel budget exists in `lctx-postgres` (`ResourceBudget::fixed/scoped/reserve`), which `lctx-model` owns and which was not examined.
- Tests: `cpg-core/tests/model_runtime.rs`.
- Would have to provide: a plan-level scan-admission hook, and one budget abstraction instead of two.

**M4-09 · Stage-input loading boilerplate**
- Location: 13 near-identical `async fn load<R: Record>` helpers (`catalog_core.rs:19`, `synthesis_preparation.rs:16`, `analytic_text.rs:16`, `semantic_execution.rs:23`, `normalize/mod.rs:20`, `structural.rs:16`, `final_coverage.rs:14`, `semantic_models.rs:16`, `catalog_selection.rs:19`, `retrieval.rs:23`, `catalog_evidence.rs:19`, `retrieval_preparation.rs:15`, `analysis_graphs.rs:64`, `analytic_embedding.rs:22`), a shared one in `consumed_rows.rs:120-140` (`stream`), and 3 `macro_rules!` copies (`catalog_core.rs:220`, `catalog_selection.rs:67`, `embedding_realization.rs:~82`).
- Capability: register a typed source table, run `SELECT * FROM "{NAME}"`, stream batches, decode into `Rows<R>`.
- Consumers: 33 `AttemptSession::open` call sites across 20 files.
- Invariants: permit-gated registration; one session per epoch; the stream drains on drop.
- Libraries inside: DataFusion SQL via `crate::sql::query`.
- Stated reason: none for the duplication. `consumed_rows::stream` exists but is used only for consumed inputs.
- Repetition: this is the repetition. A `DataFrame`/`ctx.table(name)` read is not used.
- Related: 9 `tokio::task::spawn_blocking(move || op(&data,&budget))` wrappers (`catalog_core.rs:85,225`, `analytic_text.rs:63`, `analysis_graphs.rs:121`, `catalog_evidence.rs:84`, `synthesis_preparation.rs:50`, `retrieval_preparation.rs:82`, `normalize/mod.rs:371`, `catalog_selection.rs:88`).
- Tests: per-stage `cpg-core/tests/*`.
- Would have to provide: nothing external; a single typed relation scan would remove the strings.

**M4-10 · Advisory-lock and lease protocol**
- Location: `locks.rs:1-100`, `lease.rs:1-327` (`LeaseDriver` trait, `LeaseContract::checked`, `Sqlx` driver), `cpg-core/generation_read.rs:119-150` (`Tokio` driver), `guard.rs:1-107`.
- Capability: shared reader lease (session `pg_advisory_lock_shared`), exclusive try-lock with a 1 s patience loop, attempt lock held on a dedicated closed-on-drop connection, and holder counting through `pg_locks` (`HOLDERS`).
- Invariants: lock order installation → selection → generation → attempt → relations; failure to roll back means the connection is discarded; a lost lease is terminal; the lease re-validates model and physical digests and live columns.
- Libraries inside: SQLx (`close_on_drop`, `begin`), tokio-postgres (via provider), `tokio::time::sleep`.
- Stated reason: module headers (`locks.rs:1-6`, `lease.rs:1-5`), ADR-0086:36-38.
- Repetition: the lease SQL text is run through two drivers (SQLx, tokio-postgres) via the `LeaseDriver` abstraction. The guard polls `selection_live()` every 1 s (`guard.rs:45-60`) with a 2 s timeout and a `Weak` reference.
- Tests: `tests/generations.rs`, `lifecycle.rs`, `cpg-core/tests/generation_read.rs`.
- Would have to provide: session-scoped advisory locks plus lease-liveness polling; nothing in the pinned set was identified.

**M4-11 · Serving admission, CPU capacity and cancellation**
- Location: `generations/runtime.rs:1-303` (`GenerationService`, `RequestExecution`, `execute`/`cpu`/`query`/`shutdown`), `python/lctx_storage/src/lib.rs:8-15` (worker threads 2, blocking 4), `stage_runtime.rs:30-60` (`borrowed_cpu`/`block_in_place`).
- Capability: bounded concurrency for CPU jobs and query connections, a request deadline carried through every await, cancellation-safe spawned owners, drained shutdown.
- Invariants: spawned work owns its permit and reservations until actual cleanup; a failed query is never replayed; a lost guard fails the request.
- Libraries inside: `tokio::sync::Semaphore` (two), `Notify`, `OwnedSemaphorePermit`, `timeout_at`, `spawn_blocking`, `std::sync::Mutex`, `AtomicBool`/`AtomicUsize`.
- Stated reason: module header `runtime.rs:1`; PyO3 header `serving.rs:1` ("Python numerical callbacks execute within the original generation's granted CPU lifetime").
- Repetition: `stage_runtime.rs` has its own RSS sampler (`/proc/self/status` every 20 ms, `Sampler`, `watch` channel). No `tokio_util::CancellationToken`/`TaskTracker` is used anywhere in scope (grep).
- Tests: `lctx/tests/serving_*`, `stage_runtime.rs` unit tests (`cpu_tests`).
- Would have to provide: a drain/active-count primitive (`TaskTracker`) in place of `Notify` + `active` counter + `admission` mutex; an RSS sampler would be a metrics library.

**M4-12 · Role configuration and connection options**
- Location: `lctx-postgres/src/lib.rs:95-260` (`Config`, `connect_role`), `roles.rs:1-180` (`RoleConfig`, `validate`, `connect`, `options`), `lib.rs:440-460` (`load_protected`), `cpg-core/generation_read.rs:579-612` (`driver_config`).
- Capability: parse protected JSON config (mode-0600 file check), validate URL options, enforce `sslmode=verify-full` for non-local hosts, set session options (`search_path`, timeouts, `default_transaction_read_only`), verify role and server version 18 on first connect.
- Invariants: no credential in `Debug` (manual impls); no driver or server text in errors (`lib.rs:29`); ranges asserted by hand.
- Libraries inside: SQLx `PgConnectOptions`/`PgPoolOptions`, `url`, serde, schemars.
- Stated reason: none beyond comments.
- Repetition: three option builders for one connection concept (`Config::connect_role`, `RoleConfig::options`, `driver_config` for tokio-postgres). The "local host or verify-full" check is duplicated at `lib.rs:~190` and `roles.rs:~155`. The capacity-preflight SQL (`max_connections - reserved - count(*) FROM pg_stat_activity`…) is duplicated in `roles.rs:121-142` and `reader.rs:44-60`. `config.statement_timeout_seconds` and similar get formatted into session options in three places.
- Tests: `lctx-postgres/src/lib.rs` `schema_tests`, `tests/installation.rs`.
- Would have to provide: layered configuration loading (`figment`/`config`), or a single options builder.

**M4-13 · Error taxonomy and SQLSTATE classification**
- Location: `lctx-postgres/src/lib.rs:29-85` (`Error`, `retryable`), `generations/mod.rs:69-157` (`Error`, `class`, `From<Error> for ModelError`), `failure.rs:1-182` (`FailureClass`, `sqlstate`, `Failure`), `cpg-core/lib.rs:30`, `generation_read.rs:56` (`ReadError`), `lctx/main.rs:~154-190` (`refused()` mapping errors to exit code 2).
- Capability: map driver errors to infrastructure classes, safe bounded details (SQLSTATE, constraint, table only), retryability, and the public failure kind (`lctx_storage/serving.rs:20-31`).
- Invariants: no backend text escapes; failure class is stored with a failed generation (`control.sql` CHECK is rendered from `FailureClass::ALL`).
- Libraries inside: thiserror, sqlx `DatabaseError`.
- Stated reason: `failure.rs:1-3`.
- Repetition: SQLSTATE→class is coded in `FailureClass::sqlstate` (`:~70`), `lib.rs:retryable` (`:~74`: `"40001"|"40P01"|"08*"|"57P01"`) and `mod.rs:Error::class` (`:~123-139`, with sqlx variant matching), a different set of codes each time. Two error enums named `Error` in one crate.
- Tests: `tests/generations.rs`, `lifecycle.rs`.
- Would have to provide: nothing from pinned crates for SQLSTATE classes; `sqlx::Error::as_database_error().code()` is already used.

**M4-14 · Embedding cache retry and idempotent admit**
- Location: `lctx-postgres/src/cache.rs:1-178` (`admit` loop `:103-150`, `insert_values` `:152-175`, `cached` `:65-100`), `operations.rs:1-115`.
- Capability: idempotent insert with committed-winner readback, 3 attempts, 50×n ms linear backoff, retry only on `retryable()`.
- Invariants: immutable winner (cache returns the committed value, not the candidate); a vector is validated (finite, unit norm, token cap) on write and read.
- Libraries inside: SQLx `QueryBuilder::push_values` (chunks of 32), tracing.
- Stated reason: doc comment `cache.rs:~100` ("On connection/commit ambiguity inspect the committed keys before retrying").
- Repetition: `lctx-embed` and Python `HttpEmbedder` have no retry. `locks::installation_exclusive` has another loop-and-sleep (`locks.rs:35-52`). `embedding_realization::Session::realize` (`:139`) caches one document per call (`cached(spec, &[key])`), so the 128-key chunking in `cached` is not exercised on that path.
- Tests: `lctx-postgres/tests/services.rs`, `cpg-core/tests/embedding_configuration.rs`.
- Would have to provide: retry/backoff policy with jitter, classification hook, attempt cap.

**M4-15 · Embedding HTTP client (Rust and Python twins)**
- Location: `lctx-embed/src/lib.rs:1-233` (`VllmEmbedder`, `request_body`, `parse_embeddings`, `vendor_schema`), `python/lctx_mcp/src/lctx_mcp/embedder.py:1-199` (`HttpEmbedder`, `request_body`, `parse_embeddings`, `check_vector`, `fake_vector`), `cpg-core/embedding_service.rs` (`Embedder` trait, `FakeEmbedder`).
- Capability: OpenAI-compatible `/v1/embeddings` and vLLM `/tokenize` over HTTP with response validation (count, index map, length, finiteness, unit norm, served model).
- Invariants: request bytes identical between Rust and Python (`specs/embedding/request_bodies.json`, E2), serde and pydantic strict twins held to one corpus (`responses.json`), deterministic fake vectors bit-identical in both languages (splitmix64).
- Libraries inside: reqwest 0.12.28 (timeouts: connect 10 s, request 300 s), serde_json, schemars; httpx2 2.13.1, pydantic strict, numpy.
- Stated reason: crate header (`lctx-embed` header lines 1-12) and `embedder.py:1-8` (body is sent as bytes, never `json=`, because httpx escapes non-ASCII).
- Repetition: the same vector check and fake generator exist in Rust (`lctx-model::embedding::check_vector`, `FakeEmbedder`) and Python. The Python client opens a new `AsyncClient` per `embed` call (`embedder.py:~135`).
- Tests: `lctx-embed` `dto_contract_tests`, `python/lctx_mcp/tests/test_embedder.py`, `test_embed_serve.py`.
- Would have to provide: nothing external for the byte-identity requirement; a typed OpenAI-compatible client would not give identical bytes.

**M4-16 · MCP wire envelope accounting (Python)**
- Location: `python/lctx_mcp/src/lctx_mcp/wire.py:1-344` (`response_encodings`, `negotiated_encodings`, `admit_request_id`, `admitted_failure`, `SchemaTool.run`, `CapabilityResource.read`, `register`), `python/lctx_storage/src/serving.rs:118-290` (`CountWrite`, `serialized_len`, `preflight_json`: an iterative walker over Python builtins computing a lower bound of JSON bytes with cycle detection and a reserved scratch budget), `:419-570` (`encode_request`, `encode_envelope`).
- Capability: enforce declared byte limits on the final JSON-RPC encoding (stdio line and modern HTTP forms) before and after dispatch, and convert failures to public kinds.
- Invariants: the full request id and both final encodings are admitted by the native CPU worker; no driver text crosses; failure encoding is one attempt, with no recursion.
- Libraries inside: FastMCP 4.0.5 (`Tool`, `Resource`, `ResourceTemplate`, `ToolResult`, `get_context`, `mask_error_details`), mcp / mcp_types (`serialize_server_result`, `JSONRPCResponse`), pydantic, PyO3 and pyo3-async-runtimes.
- Stated reason: the module docstring (`wire.py:1`) and the `serving.rs` header. The tool schemas come from Rust (`wire_tools`, `wire_tool`), and Python tools subclass `Tool` and override `run`.
- Repetition: `register` rebuilds `Tool` objects from Rust JSON declarations (one registered `SchemaTool`/`CapabilityTemplate` per declaration). No FastMCP middleware is used (grep: none in `python/lctx_mcp/src`).
- Tests: `test_wire_contract.py`, `test_transport_envelope.py`, `tests/current_transport.py` (849 lines), `failure_seam.py`.
- Would have to provide: FastMCP-level size limits on the serialized response, or middleware hooks for pre- and post-serialization size accounting.

**M4-17 · Numerical BM25 adapter**
- Location: `python/lctx_mcp/src/lctx_mcp/retrieval.py:1-68` (`NumericalScorer`).
- Capability: BM25S per-family indexes over Rust-supplied tokens, returning complete scores (zeros included).
- Invariants: Rust owns tokens, corpus identity and fusion; library version asserted (`bm25s0311`/`0.3.11`).
- Libraries inside: bm25s, numpy, json.
- Stated reason: docstring. This is an adapter, not a candidate unless the adoption trigger F13 (PG FTS) fires.
- Tests: `test_retrieval.py`.

**M4-18 · CLI**
- Location: `lctx/src/main.rs:1-801`, `compile_options.rs`, `database.rs`, `store.rs`, `generation.rs`, `runs.rs`.
- Capability: clap 4.6.7 derive CLI with exit codes 0/1/2/3 (`main.rs:721-737`), classification of refusals by `error.chain()` downcast (`refused()` `:~154-190`), JSON output by `serde_json::to_string_pretty` + `println!`, and hermetic `uv`/`git` subprocesses (`main.rs:~215,~268`: env scrub, `--template=`).
- Libraries inside: clap derive, anyhow, thiserror, fs-err, tokio, tikv-jemallocator.
- Stated reason: crate-header doc table lists exit status; ADR-0013/0016 are cited.
- Repetition: no `tracing_subscriber` setup in `lctx/src` (it is in `cpg-extract/src/logging.rs:10-24`, `LCTX_LOG` env filter). Hand-written validation of embedding endpoint URL (`compile_options.rs:~60-80`).
- Tests: `lctx/tests/store_cli.rs`, `acquire.rs`, `model_describe.rs`.
- Would have to provide: nothing flagged for clap itself; exit-code mapping and output formatting are small.

**M4-19 · Test-database harness**
- Location: `lctx-postgres/src/testing.rs:1-930` (`DisposableDatabase`, attempt harness), `bootstrap.rs:1-31`, `sql/database.sql`.
- Capability: disposable PG18 with the four roles, bootstrap SQL and production session limits.
- Libraries inside: testcontainers-modules, sqlx `raw_sql`.
- Stated reason: `docs/postgresql.md` ("provisioned like production"). This is a harness, expected adapter code.
- Repetition: roles are created as inline SQL in `testing.rs:54` and as `format!` in `bootstrap.rs:22`.
- Tests: used by all `real-PG` suites.

**M4-20 · Operations/attempt log**
- Location: `lctx-postgres/src/operations.rs:1-115` (`start_attempt`, `event`, `runs`, `events`).
- Capability: idempotent attempt and event rows with `ON CONFLICT DO NOTHING` plus re-read verification; paged reads.
- Libraries inside: SQLx `FromRow`.
- Related: F7 (notifications/jobs) states no worker consumer. This is domain bookkeeping, not a candidate.

## 2. SQL composition survey

### Counts

| Group | Count |
|---|---|
| `AssertSqlSafe` in `crates/*/src` | 46, all `lctx-postgres` (`generations/mod.rs` 5, `vocabulary.rs` 13, `verify.rs` 9, `install.rs` 4, `receipts.rs` 3, `stage_validation.rs` 2, `lease.rs` 2, `catalog.rs` 2, `evidence_service.rs` 1, `testing.rs` 2, `bootstrap.rs` 1) |
| `AssertSqlSafe` in `crates/*/tests` | 288 (cpg-core 199, lctx-postgres 59, lctx 30) |
| Static `sqlx::query(…)`/`query_as`/`query_scalar` text | about 222 across `lctx-postgres/src` (grep count, all binds) |
| `sea_query` builders | `ddl.rs:228-410` only |
| `QueryBuilder::push_values` | `cache.rs:156` only |
| `sqlx::query!` macros | 0 |
| DataFusion `format!("SELECT * FROM \"{NAME}\"")` in `cpg-core/src` | 19 sites (+1 with `ORDER BY id`: `analysis_report.rs:78`; 3 in macros) |

### Purposes

**DDL generation (identifiers from the model; one literal context)**
- `ddl.rs` (58 `format!`): CREATE SCHEMA (`:131`), table (sea-query, `:228-410`), FKs (sea-query), CREATE VIEW (`:420-430`, `:455-460`, `:590-600`, `:617-630`), CREATE INDEX (`:466-476`), GRANT/REVOKE (`:24-37`, `:488-500`, `:651`), CTAS WITH NO DATA (`:636-650`), `ALTER TABLE … ADD CONSTRAINT` (`:655-665`).
- Interpolated: `quoted(schema)`, `quoted(relation.name())`, `quoted(field.name())`. Inlined as literals: generation hex in `decode('…','hex')` (`:240-248`), `literal(rule.rule)` and `literal(premise.name())` (escaped by `replace('\'', "''")`, `:549`), integer codes and tags, hashed constraint/index names (`ContentHash` hex prefix).
- Executed via `execute(connection, Vec<String>)` (`mod.rs:1166`) with `AssertSqlSafe`.
- `control()` (`:155-212`) templates `control.sql` with `str::replace("{token}", …)`: `{control}` is quoted, and the list tokens (`{lifecycle}`, `{classes}`, `{profiles}`, `{frontiers}`) are wrapped in quote marks with no escaping because the sources are constants.

**Catalog introspection (static text, bound parameters)**
- `verify.rs:561-596` `DESCRIBE` (bind schema name), `:385-540` roles/database/inventory, `ddl.rs:106` `LIVE_COLUMNS` (bind schema), `serving_shape.rs:55` (`pg_get_viewdef` by `to_regclass($1)`), `vectors.rs:404-652` (5 queries on `lctx_cache` objects, constant names), `roles.rs:121`, `reader.rs:44` (capacity), `lib.rs:285-300` (`OwnerPool::verify`), `locks.rs` (`pg_locks`).
- None interpolates untrusted values.

**Lifecycle statements with identifiers**
- `LOCK TABLE {qualified} IN ACCESS EXCLUSIVE MODE`: `receipts.rs:64`, `:180`, `:358`, `vocabulary.rs:~187`.
- `DROP SCHEMA {quoted} CASCADE`: `mod.rs:~469`, `install.rs:143`, `:164`. `DELETE FROM lctx_model_store.{table}` where `table` is from the constant array `CONTROL_RECORDS` (14 names; `mod.rs:~718`; used at `mod.rs:~477`, `install.rs:153`) and a `SELECT EXISTS(…) OR EXISTS(…)` composed from the same array (`mod.rs:431`).
- `REVOKE INSERT ON … FROM lctx_importer`: `vocabulary.rs:~106,~112`, `ddl::completed_output`.
- `SAVEPOINT`/`ROLLBACK TO SAVEPOINT` are static literals.

**Dynamic projection and relation reads (SQLx path)**
- `mod.rs:1041-1130` (`visit_physical_inner`): `SELECT [DISTINCT ]{quoted columns} FROM {qualified} [WHERE {quoted}=ANY($1)] ORDER BY {quoted [COLLATE "C"]}`. Identifiers come from the model; filter values are bound.
- `evidence_service.rs:498`: `SELECT count(*) FROM {qualified} WHERE id=ANY($1)` (relation name checked by `check_relation`).
- `vocabulary.rs:252-268`: delta-vs-base conflict, `INSERT … SELECT DISTINCT {columns},{epoch}::smallint FROM {delta} ON CONFLICT(generation_id,id) DO NOTHING`; `{epoch}` is an integer from a typed prefix ordinal (inlined, not bound).

**Nominal reference checks:** see M4-03 (`vocabulary.rs:299,331`, `stage_validation.rs:92,250`). `FROM` names come from `physical()`/`input_physical()`, so they are model-derived hashes.

**Generated relation reads (DataFusion path, `cpg-core`)**
- `SELECT * FROM "{NAME}"` (20 sites) is an unescaped quoted constant (`R::NAME` is a `&'static str` from the model) planned through `sql::query`, which sets `SQLOptions` to refuse DDL/DML/statements. The ast-grep rule `rules/sql-through-helper.yml` forbids `ctx.sql(`/`ctx.sql_with_options(` outside `sql.rs`.
- `generation_read.rs:783-790`: `SELECT {quote(col)…} FROM {source} [WHERE (unparsed pred) AND …] [LIMIT {n}]`, executed on a leased tokio-postgres connection. Predicates come from `Unparser<PostgreSqlDialect>` over admitted expressions with binary literals rewritten to `'\x…'::bytea` casts (`postgres_expr`, `:695-720`); `standard_conforming_strings=on` is forced in session options (`:604`).
- `model_runtime.rs:~139`: `policy.select_sql(|name| format!("\"{name}\""))` (call-policy views registered as DataFusion logical views); the same `select_sql` is also lowered to PostgreSQL by `ddl.rs:420-430` with a different quoting closure.

**Catalog summary reads:** `catalog.rs:90-115` composes a constant `SUMMARY` with static `WHERE` strings; `AssertSqlSafe` is required only because the result is a `String`.

**Bootstrap/test:** `bootstrap.rs:22` (password into `CREATE ROLE`; guarded by an alphanumeric check `:18`), `testing.rs:54,:571`. Test code at 288 sites is expected.

### Escaping summary
- Identifier quoting is hand-written in two crates (`mod.rs:1172`, `generation_read.rs:614`).
- Literals: `ddl::literal` is the only one, used for rule and premise names. All other literals are numeric.
- Bound parameters carry all variable filter values (`ANY($1)`, `$1..`).
- `QueryBuilder` is used once.

## 3. Trigger evidence

| Trigger (forward plan §7) | Observed facts |
|---|---|
| F4 dynamic SQL via SeaQuery: "substantive typed variable joins/expressions beyond clear SQLx queries" | sea-query already imported for DDL (`ddl.rs:12`) and listed as such in ADR-0086:40. Generation-bound runtime SQL is `format!` text in 14 groups (§2): 9 in `vocabulary.rs`, 9 in `verify.rs` (DDL replay), 3 reference checks, plus 20 DataFusion strings. The `sql-through-helper` rule polices the DataFusion side only. `no-string-sql-in-lctx-mcp.yml` bars SQL strings in Python. No `Query::select()` is used anywhere. |
| F6 Python SQLAlchemy/psycopg: "Python gains its own relational workflow" | Python has no database client: `python/lctx_mcp/pyproject.toml` depends on fastmcp, pyarrow, numpy, httpx2, bm25s, pydantic, mcp-types, `lctx-semantics`, `lctx-storage`. All database work is through the PyO3 `lctx_storage` bridge. `no-string-sql-in-lctx-mcp.yml` enforces this. Trigger not met by code facts. |
| F7 notifications/jobs: "actual worker or polling-cost consumer" | One polling loop: `guard.rs:45-60` (1 s `selection_live()` per guard, 2 s timeout). `locks.rs:35-52` busy-waits 50 ms up to 1 s. No `LISTEN`/`NOTIFY`, no queue tables, no worker. `operations.rs` is a log. |
| F11 ADBC / F12 DataFusion client protocols / F8 pgrx | No ADBC or pg-wire. `lctx query` is an in-process CLI over `InspectionSession`. PG→Arrow reads go through the forked `datafusion-table-providers-postgres` (`query_arrow_bounded`) and sqlx rows (`codec.rs`). |
| F13 PostgreSQL FTS / alternate vector engine | Lexical scoring is bm25s in Python over Rust tokens (`retrieval.rs`). Vector scoring is pgvector exact cosine in SQL (`vectors.rs:354`, `OPERATOR(lctx_ext.<=>)`). Fused in Rust. |
| Moka / request cache | No cache crate (grep: no `moka`, `lru`, `DashMap`) in scope. Two bespoke caches: `Store::cached/admit` (PostgreSQL embedding cache) and the prepared `AdmittedSelection` / `VectorArtifact` held by `GenerationService` (single instance per generation guard). No measured cost recorded in these files. |
| Cornucopia/driver alternatives | SQLx is the owner of 222 static queries; tokio-postgres 0.7.18 is used only inside the provider path (`generation_read.rs`) with the `LeaseDriver` bridge. |
| `postgres-types` | Not used. |
| tracing/logging | `tracing::` appears 6 times in the scoped Rust (`cache.rs`, `guard`/`stage_runtime`). The subscriber is initialized in `cpg-extract/src/logging.rs` (outside scope). The CLI does not configure tracing (`lctx/src`). `datafusion-tracing` is a listed skill, but the crate was not found in `Cargo.toml` greps within scope. |
| Retry/backoff | One bespoke loop (`cache.rs:103-150`); 0 uses of a retry crate (grep for `tower`, `backoff`, `retry` returns only `cache.rs`/`lib.rs`). |
| Cancellation | Tokio `JoinHandle::abort`-style plus drop semantics. 0 `CancellationToken`/`TaskTracker` in scope. |

## 4. Adapter or domain, not candidates

- `lctx-postgres/src/generations/lifecycle.rs`: attempt/sealed/validated state machine over owner connection and attempt advisory lock. Domain lifecycle policy on SQLx.
- `generations/receipts.rs`, `vocabulary.rs` (publication logic), `stage_validation.rs`, `publication_validation.rs`, `validation_views.rs`: domain validation policy (SQL text noted in §2/M4-03).
- `generations/selection.rs`, `retrieval_service.rs`, `catalog_service.rs`, `packet_service.rs`, `packet_reads.rs`, `capture_packets.rs`, `evidence_service.rs`, `capability_service.rs`, `native_service.rs`, `flow_inventory_service.rs`, `source_*_service.rs`, `access_routes_service.rs`, `operation_sections.rs`: serving services that read typed rows from a lease and run domain logic. They use static SQL with binds only.
- `generations/service.rs`, `catalog.rs` (summary and list): thin service and summary reads.
- `generations/install.rs`: install, reset, plan orchestration over M4-01/M4-04.
- `generations/codec.rs`: effect-boundary adapter (see M4-06).
- `lctx-postgres/src/operations.rs`, `cache.rs`: bookkeeping and cache domain (retry loop noted at M4-14).
- `lctx-postgres/src/testing.rs`, `bootstrap.rs`: harness.
- `cpg-core/src/compilation.rs`, `facts.rs`: pipeline and stage orchestration (domain).
- `cpg-core/src/stage_runtime.rs`: stage measurement and `borrowed_cpu` (RSS sampler is generic, noted at M4-11).
- `cpg-core/src/semantic_execution.rs`, `semantic_models.rs`, `semantic_summaries.rs`, `local_semantics.rs`, `analytic*.rs`, `analysis_*.rs`, `catalog_*.rs`, `retrieval*.rs`, `synthesis*.rs`, `structural.rs`, `final_coverage.rs`, `normalize/*`, `embedding_realization.rs`: stage producers; their I/O follows M4-09. Compute bodies live in `lctx-model` and were not read.
- `cpg-core/src/sql.rs`: single read-only SQL gate (22 lines), `SQLOptions`, adapter.
- `lctx-analytics/src/*`: re-export shims (15 lines).
- `lctx-workspace-hack`: Hakari-generated feature unification, not read.
- `python/lctx_semantics` (`src/lib.rs`, `python/lctx_semantics/__init__.py`): thin PyO3 wrappers over `lctx_model::domain::serving`.
- `python/lctx_storage/src/serving.rs` aside from `preflight_json`/`CountWrite`: PyO3 bridge (async via `pyo3_async_runtimes::tokio::future_into_py`, `py.detach`).
- `python/lctx_mcp/src/lctx_mcp/server.py`, `__main__.py` (argparse, 71 lines), `generation.py`: FastMCP assembly, CLI wiring and lifespan. Python CLI uses `argparse`, not typer.
- `lctx/src/propose.rs`, `model.rs`, `compile.rs`, `serving.rs`, `query.rs`, `runs.rs`, `store.rs`, `generation.rs`: CLI subcommand handlers.

## 5. Search coverage and limits

- **Searched:** all of `crates/cpg-core/src`, `crates/lctx-postgres/src`, `crates/lctx/src`, `crates/lctx-embed/src`, `crates/lctx-analytics/src`, `python/lctx_mcp/src`, `python/lctx_storage/src`, `python/lctx_semantics/src` and `python/lctx_semantics/python`. Patterns used: `AssertSqlSafe`, `raw_sql`, SQL keyword greps in `format!`, `sea_query`, `quoted(`, `Semaphore`, `CancellationToken`, `sleep`, `retry`, `spawn`, `timeout`, `moka|lru|DashMap`, `tracing_subscriber`, `Command::new`, `thiserror`, `enum .*Error`.
- **Read in full or near-full:** `ddl.rs`, `codec.rs`, `locks.rs`, `guard.rs`, `runtime.rs`, `failure.rs`, `lease.rs`, `lib.rs`, `cache.rs`, `operations.rs`, `roles.rs`, `bootstrap.rs`, `generation_read.rs`, `model_runtime.rs`, `prepared.rs`, `stage_runtime.rs`, `sql.rs`, all `python/lctx_mcp/src/*.py`, `lctx-embed/src/lib.rs`, `embedding_service.rs`.
- **Read in part:**
  - `verify.rs` (not lines 200-255 or 380-385).
  - `vocabulary.rs` (about 40%).
  - `mod.rs` (lines 1-330, 425-485, 560-740 and 1040-1209).
  - `receipts.rs`, `stage_validation.rs`, `publication_validation.rs` (about 30% each).
  - `vectors.rs` (headers and queries only).
  - `lifecycle.rs`, `selection.rs`, `main.rs` of `lctx` (first 200 lines plus 395-470).
  - `compile_options.rs`, `normalize/mod.rs` (the tail).
  - `serving.rs` in `lctx_storage` (lines 1-300 only).
- **Not read:** service files listed in §4, `testing.rs` harness bodies, `lctx-model` (`ResourceBudget`, `Record::decode`, `stages`, `serving`), `cpg-extract` (logging, `library.rs` subprocesses), `control.sql` and migrations contents, `cpg-core` stage computation bodies, and all Python tests beyond header skims.
- **Docs consulted:** ADR-0086, `docs/postgresql.md:160-175`, `storage-and-publication.md:125-145`, forward plan §7 (lines 1130-1205), `rules/*.yml`. Not read: `synthesis-and-serving.md`, ADR-0069/0073/0078 text. ADR-0068 and ADR-0069 do not exist in `docs/adr/`, so the "SQLx owner" claim rests on ADR-0086:40 and the storage section.
- **Counts are grep-derived:** the 222 static-query figure counts `sqlx::query`/`query_as`/`query_scalar`/`raw_sql` occurrences in `lctx-postgres/src` by line. Per-file `format!` SQL counts are by keyword match on source lines. The 46 and 288 `AssertSqlSafe` counts are exact line counts.
- **Absence claims** (no `CancellationToken`, no `moka`, no `tower`/`backoff`, no `Query::select`, no `query!`) hold for the scoped source trees and the root `Cargo.toml` greps above, not for `lctx-model`, `cpg-extract` or `cpg-flow`.
- **Unchecked:** whether sea-query 1.0.2 with `default-features=false` and `backend-postgres`/`postgres-array` exposes the view, index and grant builders. The library researchers need to confirm that.
- **Nothing was built or run.** No test outcome is claimed.
