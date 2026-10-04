# L1: compute and storage libraries (DataFusion/Arrow, sqlx/sea-query/pgpq, tokio, contracts)

Lane L1 of the 2026-10-04 library-leverage review. Library-side research: which surface of the
already-pinned compute/storage family the workspace does not use, its contract and limits, and its
fit to named consumers. Adoption and architecture are the coordinator's and design reviewer's.

**Baseline.** `main` at `948b2a88` (2026-10-04). Static inspection of pinned sources in
`~/.cargo/registry/src/index.crates.io-*/` (versions as named per row) and the git checkout of the
table-provider fork (`~/.cargo/git/checkouts/datafusion-table-providers-eb0d24de1d6f0953/a41da22`),
plus the `sqlx-postgres`, `datafusion-tracing` and `serde-arrow` skills (briefs reviewed 2026-09-29,
probes against PostgreSQL 18). Upgrade deltas: crates.io API and upstream release notes, checked
2026-10-04. **No probe was run** in this lane (`not_run`); every claim is source/static unless it
cites a skill probe ID (`B…`/`C…`), which is the skill's own executed evidence at the same pins.
Labels: `Observed` (source read at the pin), `Probed (skill)`, `Proposed` (fit, not decided).

## 1. Capability table

| Library · pinned item | Contract and limits (source) | Repo consumers | Fit: replaces / enables · does not preserve |
|---|---|---|---|
| **sea-query 1.0.2** `Query::select/insert/update`, `Expr`, `.build(PostgresQueryBuilder)` → `(String, Values)` | Deterministic rendering, every identifier double-quoted with embedded `"` doubled (`Probed (skill)` B030/B031). `to_string` inlines values as `E'..'` literals; `build` gives `$n` + `Values` (B036). Binding `Values` to sqlx 0.9 needs `sea-query-sqlx` (0.9.1, 2026-05-30, **not pinned**) or hand binding. No GRANT/REVOKE, `CREATE SCHEMA` or `CREATE VIEW` builders exist in 1.0.2 (`grep` of `src/` for grant/schema-create/view: none). | `ddl.rs` (only sea-query user: `Table::create`, `ForeignKey`, `Index`, `Expr::cust` for CHECKs, lines 228–410); 14 `format!` DDL/DCL sites in `ddl.rs`; `install.rs`, `vocabulary.rs`, `lease.rs` `format!` SQL | Could absorb table/index/FK/insert/select text now hand-built. Would **not** cover GRANT/REVOKE, CREATE SCHEMA/VIEW (stay text) nor the many `Expr::cust` CHECK bodies (verbatim text either way). Rendering changes alter `physical_digest` (§2.1). |
| **sea-query 1.0.2** feature `sqlx-utils` (not enabled): `sea_query::sqlx::postgres::{query!, query_as!}`; always-on `raw_query!` | `raw_query!(PostgresQueryBuilder, "… {a} … IN ({..v}) … VALUES {..(rows.0:1),}")` renders `$n` placeholders with named params, array and tuple expansion (README §4, `src/raw_sql.rs`). `sqlx-utils` expands to `sqlx::postgres::query(sqlx::AssertSqlSafe(..)).bind(..)` (`sea-query-derive-1.0.0/src/sqlx.rs:80`), i.e. targets sqlx 0.9 directly, no `Value` binder needed. Placeholders only for values; identifiers inside the template are still author text. | Same as above; the F4 trigger (forward plan §7 "PostgreSQL adoption triggers", F4) names "SeaQuery+binder with allowlisted identifiers and bound values" | A binder path that needs **no new crate** (feature flag on the pinned crate). Not executed by the skill (feature off in the indexed profile): `Proposed`, needs a compile check. |
| **sqlx 0.9.0** `SqlSafeStr`/`AssertSqlSafe`, `QueryBuilder::{push_bind, separated, push_values, push_tuples}` | `AssertSqlSafe` is a lifetime marker, not an injection check; a leaked `&'static str` bypasses it (C002); `copy_in_raw` takes plain `&str` (C003). `push_bind` sends parameters; `push` splices raw text; 65,535-bind ceiling (B017, B027). **No identifier-quoting helper** in sqlx-core/sqlx-postgres 0.9.0 (grep `quote`: none). | 46 production `AssertSqlSafe` sites, all in `crates/lctx-postgres/src` (`verify.rs` 10, `vocabulary.rs` 14, `mod.rs` 5, `install.rs` 4, …); `cache.rs:156` (one `QueryBuilder`); `mod.rs:633` `copy_in_raw(&format!(..))` (unchecked by sqlx) | QueryBuilder suits variable `IN`/VALUES lists with bound values; it does nothing for identifiers. Two repo quoting helpers duplicate each other: `generation_read.rs:614 quote` and `generations/mod.rs:1172 quoted` (same body). `ddl.rs:280` quotes `field.name()` without doubling, relying on model identifier validation (`model.rs:477`, `models.rs:901`). |
| **sqlx 0.9.0** `query!`/`query_as!` + offline `.sqlx` | Compile-time typing against a live DB or `.sqlx`; with no `DATABASE_URL` reads `.sqlx` even without `SQLX_OFFLINE` (C006); expressions are `Option` unless aliased `"col!"` (C007). Cannot check per-generation schemas whose names are computed at run time. | `.sqlx/` exists and is **empty**; `.cargo/config.toml:17` sets `SQLX_OFFLINE=true`; 0 macro uses; 217 `sqlx::query*`/`raw_sql` calls in `lctx-postgres/src` | Applicable only to fixed-schema statements (`lctx_cache`, `lctx_ops`, `public._sqlx_migrations`, control schema if lowered before `prepare`). Generation-schema reads are out of reach. Needs a `cargo sqlx prepare` workflow (sqlx-cli is not pinned). |
| **DataFusion 55.1.0** `datafusion::sql::unparser::{Unparser, plan_to_sql, expr_to_sql}`, `dialect::PostgreSqlDialect` | `PostgreSqlDialect::identifier_quote_style` always `Some('"')` (`datafusion-sql-55.1.0/src/unparser/dialect.rs:386`); sqlparser 0.62.0 `Ident` Display escapes the quote char by doubling (`sqlparser-0.62.0/src/ast/mod.rs:377-388`). String literals render single-quoted with `'` doubled; correctness assumes `standard_conforming_strings=on`, which `generation_read.rs:608` sets. Arrays render `ARRAY[..]`. | Already used for pushdown predicates: `generation_read.rs:767-781` (`Unparser::new(&PostgreSqlDialect{})`), with an admitted-expression algebra (`domain`, `comparable`) | Could render the column list/`FROM` too (`plan_to_sql` of a `TableScan`+`Filter`+`Limit`), removing `quote`/`format!` at 783-789. Equivalence of DataFusion vs PostgreSQL semantics (collation, NULL, numeric) remains the repo's admission job; the unparser guarantees syntax, not meaning. |
| **DataFusion 55.1.0** `SessionContext::table(name)` / `DataFrame` API | Builds a `LogicalPlan` with no SQL text; same providers, same execution. | ~20 identical `session.query(&format!("SELECT * FROM \"{}\"", R::NAME))` reads in `cpg-core/src` (`structural.rs:31`, `catalog_core.rs:26,220`, `catalog_selection.rs:26,67`, `normalize/mod.rs:22`, `retrieval.rs:30`, `analysis_report.rs:78` with `ORDER BY id`, …) | Removes string SQL from scan reads. Note `sql.rs` (`read_only()` options) and `rules/sql-through-helper.yml` are the single SQL gate; a DataFrame path parses no SQL so it cannot carry DDL/DML, but it would bypass the helper the rule polices; `PreparedQuery::from_frame` (`model_runtime/prepared.rs`) already accepts a `DataFrame`. |
| **DataFusion 55.1.0** relational operators (joins, aggregates, windows, `WITH RECURSIVE`, UDF/UDAF/UDTF), `MemTable` | `RecursiveQueryExec`: "no limit or checks applied to detect an infinite recursion" (`datafusion-physical-plan-55.1.0/src/recursive_query.rs:64`); `UNION` (distinct) form deduplicates across iterations (`DistinctDeduplicator`, line 458) and charges a `MemoryReservation("RecursiveQuery")` (line 316). `enable_recursive_ctes` default true (`datafusion-common-55.1.0/src/config.rs:989`). `HashJoinExec` still does not spill (only a comment at `hash_join/exec.rs:2265`); nested-loop and sort-merge joins have spill code. | **No production join/aggregate/window use found** in `cpg-core/src` or `lctx/src` (grep for `.join(`, `.aggregate(`, `JoinType`, non-trivial SQL: only the scans above plus `model_runtime.rs:153`, which runs `EventPolicy::select_sql`). | DESIGN B3 text ("Joins, projections and unions construct suitable normalized and analysis relations", `DESIGN.md:285-289`) and ADR-0085 ("DataFusion retains relational compute") describe more DataFusion compute than the code shows: normalization/analysis joins happen in Rust over decoded records (`lctx-model`). Observation for the reviewer, not a defect claim. |
| **Arrow 59.3.0** `arrow-row` (`RowConverter`, `Row`/`OwnedRow: Ord + Hash`), `arrow-select` (`take`, `filter`, `interleave`, `concat`, `zip`) | Direct deps of `lctx-model` today. `Row`/`OwnedRow` implement `Ord`, `Eq`, `Hash` (`arrow-row-59.3.0/src/lib.rs:1578-1652`): usable as keys of `HashMap`/`BTreeMap` for multi-column grouping/dedup without DataFusion. Row bytes are not a stable encoding across arrow releases (already recorded in the 2026-09-23 review for `IdHasher`); ordering follows `SortOptions`, byte-wise for strings. | `memory.rs:547 order` (`RowConverter` + stable `sort_by` + `take_record_batch`), `memory.rs:675 unique`; `concat_batches` ×6 | Already the right kernel family. A key-in-memory relational join/group in `lctx-model` can use `OwnedRow` keys instead of bespoke tuple keys **only** where the key is not persisted or hashed into identity. |
| **Arrow 59.3.0** `arrow-ord` (`sort_to_indices`, `lexsort_to_indices`, `partition::partition`, `rank::rank`, `cmp::*`) | In `Cargo.lock` (via DataFusion) but **not a direct dep** of `lctx-model`; adding it keeps the single Arrow version (`just deps`). `lexsort_to_indices`/`sort_to_indices` are **unstable sorts** (`arrow-ord-59.3.0/src/sort.rs:45,137`, `sort_unstable_by` at :73 of the lexsort path). `partition` returns ranges of equal adjacent rows. | `memory.rs:546` documents "Ties keep their prior order" | `lexsort_to_indices` would break that stability contract unless a row-index tie-breaker column is appended; current code is correct and comparably short. `partition(&[id])` could replace the adjacent-id loop in `unique` (marginal). |
| **DataFusion 55.1.0** `MemoryPool`/`MemoryReservation`, `PeakRecordingPool`, `TrackConsumersPool<GreedyMemoryPool>` | `with_memory_limit` builds `TrackConsumersPool(GreedyMemoryPool, top 5)` (`runtime_env.rs:430-436`); `PeakRecordingPool` records reset-able and lifetime peaks (`memory_pool/peak_recording.rs:86`). Reservations are cooperative: only bytes an operator registers are counted; untracked Arrow allocations are invisible; greedy pool is first-come, not fair. | **Already adopted**: `model_runtime.rs:57-70` builds the pool; `ComputePool`/`ComputeReservation` (`model_runtime.rs:310-368`) make the model's `ResourceBudget` a view of it; `generation_read.rs:865` registers scans. | Attempt-budget charging (`charged.rs` `StateCharge`, `NODE_ALLOWANCE` estimates; `batching.rs` row admission) is the repo's *declared* accounting layered on the same pool. DataFusion would not supply semantic owners, refusal reasons or the "unbound refuses" default. Disk manager is `Disabled` (`model_runtime.rs:63`), so spill pools are moot. |
| **sqlx-postgres 0.9.0** `PgAdvisoryLock` | Session-level `pg_advisory_lock`/`pg_try_advisory_lock` only, exclusive only; guard queues `pg_advisory_unlock` on drop (`advisory_lock.rs:185-325`). No shared, no transaction-scoped variant. | `locks.rs` uses `pg_advisory_xact_lock_shared`, `pg_try_advisory_xact_lock`, two-key generation/attempt locks, `pg_locks` holder counts | **Does not fit**: the repo's protocol needs xact-scoped and shared modes. Keep raw SQL. |
| **sqlx-postgres 0.9.0** `PgListener` | `listen/listen_all/recv/try_recv/into_stream`, `eager_reconnect`; notifications are lossy across reconnects. | None (forward plan F7: "LISTEN/NOTIFY is a reconnectable hint") | Available when F7 triggers; not a truth channel. |
| **sqlx 0.9.0** `Migrator` / `migrate!` | Checksums, VersionMissing/VersionMismatch; **on error returns before `pg_advisory_unlock`, the lock stays on that connection** (`Probed (skill)` B024). No schema diffing. | `lctx-postgres/src/lib.rs:25,394` `MIGRATOR.run(&self.inner.pool)`; pre-check covers missing versions (`LegacyHistory`) but not checksum mismatch | Observation: on VersionMismatch the lock-holding connection returns to the **pool**. Likely low impact for a short-lived CLI; the skill's implementation note is "run on a dedicated connection and close it after an error". Shadow-install diffing (`verify.rs`) has no pinned library alternative (sqlx migrate does not diff; none of the pinned crates diffs catalogs). |
| **sqlx 0.9.0** pool hooks `after_connect`, `before_acquire`, `after_release`, `acquire_slow_threshold`, `test_before_acquire` | `pool/options.rs:173-492`. `SET` on `&pool` may hit another session (B029); hooks apply per connection. | `runtime.rs:286` sets `statement_timeout` per lease via `set_config`; role config in `roles.rs` | Fits session settings that are invariant per role (search_path, read-only, timeouts) via `after_connect`; per-request deadlines stay explicit. |
| **sqlx 0.9.0** cancellation | No client-side `CancelRequest` API (only backend key data is parsed, `message/backend_key_data.rs`). Dropping a query future leaves the server statement running; the connection drains on next use (`connection/mod.rs:94 wait_until_ready`). `tokio-postgres 0.7.18` (pinned, used by `cpg-core`) has `Client::cancel_token()`. | `runtime.rs` relies on `statement_timeout` + task ownership; `generation_read.rs` uses tokio-postgres through the fork pool | Server-side cancel exists only on the tokio-postgres path. Unknown whether the fork's pool exposes the `Client` for `cancel_token` (not checked). |
| **pgpq 0.12.0** `ArrowToPostgresBinaryEncoder`, `EncoderBuilder` | Arrow→binary COPY only (no decoder). Target column type must match encoding (UInt32→INT8, B041); cannot write pgvector (B047); COPY must say `BINARY` (B048). | `generations/mod.rs:600-670` per-row slice encoding with per-row wire admission | Already used. No pinned decoder for binary `COPY TO`; reads stay row-decoded (`codec.rs`). |
| **datafusion-table-providers-postgres** (fork a41da224) `rows_to_arrow`, `query_arrow_bounded` | `crates/postgres/src/arrow_sql_gen/mod.rs:312`; inferred schemas lose widths (bytea→Binary, B073); `declared_table` takes an explicit schema. | `generation_read.rs:34,880` (scan path) | Two PG→Arrow decoders coexist: this one (tokio-postgres) for DataFusion scans and `codec.rs` (sqlx) for store reads. Converging on one needs one driver; out of L1 scope. |
| **serde_arrow 0.15.1** (`arrow-59`) | `ArrayBuilder::from_arrow(fields)` / `from_record_batch` with explicit schemas; misnamed nullable fields become silent nulls; `None` into non-nullable errors; failed push poisons the builder (skill). | **Adopted**: generated `encode`/`decode` (`lctx-model-macros/src/lib.rs:474-500,838-850`), with exact-schema equality check on decode. `ArrowColumn` no longer exists (0 hits). | `codec.rs` (`PgRow`→Arrow per `DataType`) could go through the generated physical struct only for typed reads; it serves `Relation`-generic reads, so serde_arrow does not displace it without a typed row source. |
| **schemars 1.2.2** derive (`deny_unknown_fields`) + **jsonschema 0.58.2** | Derive for externally tagged enums emits `oneOf` of `{"type":"object","properties":{V:…},"required":[V],"additionalProperties":false}` (`schemars-1.2.2/src/_private/mod.rs:246-257`); with container `deny_unknown_fields` the variant body also closes (`schemars_derive-1.2.2/src/schema_exprs.rs:264-282`). | Hand-written `enum_schema!` in `lctx-model/src/domain/serving/schema.rs:128-160` for `StructuralType`, `RelationTarget`, `FieldTarget`, `FacetValue`, `Predicate`, …; derives elsewhere (`serving/responses.rs`, `native_requests/mod.rs`, `lctx-embed`, `roles.rs`) | `Proposed`: `#[derive(JsonSchema)] #[schemars(deny_unknown_fields)]` would generate the same shape and track variants by construction (the macro's exhaustiveness guard becomes unnecessary). Must keep hand impls for `Id<T>`/`ArmId`/`ContentHash` (custom `x-nominal-*` keywords). Exact-output equality is unverified: snapshot before switching. `#[serde(deny_unknown_fields)]` would also make runtime decode match the schema (stricter); `#[schemars(..)]` changes the schema only. |
| **postcard 1.1.3** | Canonical, compact serde encoding; `experimental::serialized_size`. | Adopted: `serving/identity.rs:50`, `dispatch.rs:36`, `projection/snapshot.rs:247-280`, `cursor.rs:29,58` | No gap found. |
| **tokio 1.53.1** `Semaphore`, `Notify`, `JoinSet`, `task::spawn_blocking`, `time::timeout_at` | Standard. | `runtime.rs` (owned permits, `Notify` drain with `AtomicUsize`, `admission` mutex, spawned query task owning cleanup) | Already the right primitives. |
| **tokio-util 0.7.19** (in graph, `lctx-workspace-hack/Cargo.toml:114` features `codec`,`io`) | `sync::CancellationToken` is **not feature-gated** (`tokio-util-0.7.19/src/lib.rs:57`, `sync/mod.rs:4`). `task::TaskTracker`, `AbortOnDropHandle`, `JoinMap` need feature `rt` (`task/mod.rs:6-25`), which is not enabled anywhere. | `runtime.rs` hand-rolls stop-flag + `active` counter + `Notify` drain (`shutdown`, lines 168-187); no `CancellationToken`/`TaskTracker` use anywhere in production | `TaskTracker` (`close()` + `wait()`) matches the "stop new work, drain actual jobs" shape; it would need a direct dep and the `rt` feature. It tracks tasks, not `RequestExecution` grants that cross `spawn_blocking` and survive caller cancellation, so it would replace the counter only if every grant is a tracked task (`Proposed`, not checked line by line). |
| **tracing 0.1.44 / tracing-subscriber 0.3.23** | `EnvFilter`, fmt layer. | `cpg-extract/src/logging.rs:10-12` (`LCTX_LOG`); 1 span-construction site, 5 event macros in production crates | Spans are nearly unused; reporting is via receipts/errors. |
| **datafusion-tracing 55.0.0** (not linked) | Requires `datafusion = "55.0.0"`, `default-features = false` (compatible with 55.1.0). Runtime deps: async-trait, comfy-table, datafusion, delegate, futures, pin-project, similar, tracing, tracing-futures, unicode-width (`datafusion-tracing-55.0.0/Cargo.toml`). **No OpenTelemetry in its runtime dependencies**; OTel crates appear only in the skill's wiring set. New to the lock: `delegate`, `tracing-futures` (others present). | None | Would give per-operator spans/metrics/previews for the DataFusion scans; value is low while DataFusion executes scans only. |

## 2. Questions

### 2.1 SQL composition (Q1)

- **What the sites are** (`Observed`): (a) ~20 `cpg-core/src` sites are the same idiom, a whole-relation
  scan `SELECT * FROM "<R::NAME>"` through the read-only helper (`sql.rs`), where `R::NAME` is a
  validated model identifier; (b) one pushdown scan builder (`generation_read.rs:767-789`) which
  already uses the DataFusion Postgres unparser for predicates; (c) PostgreSQL DDL/DCL lowering in
  `ddl.rs` (CREATE SCHEMA/VIEW, GRANT/REVOKE, CHECK bodies, derivation views) plus sea-query
  `Table::create`; (d) lifecycle/vocabulary/verify statements in `lctx-postgres` (46 `AssertSqlSafe`);
  (e) one dual-engine SQL text, `EventPolicy::select_sql` (`lctx-model/src/domain/normalized/events.rs:57`),
  rendered for PostgreSQL views (`ddl.rs:424`) and run by DataFusion (`model_runtime.rs:153`).
- **sea-query 1.0.2**: covers (c) partially (tables, indexes, FKs; CHECK bodies remain `Expr::cust`
  text) and DML/select; does **not** cover GRANT/REVOKE/CREATE SCHEMA/CREATE VIEW. Identifier
  quoting is total and doubling (B031). For bound values with sqlx 0.9: the off-by-default
  `sqlx-utils` feature of the pinned crate (`sea_query::sqlx::postgres::query!`) or `raw_query!` +
  manual binding, or the unpinned `sea-query-sqlx` 0.9.1.
- **sqlx 0.9**: no identifier quoting; `QueryBuilder::push_bind` for values; `AssertSqlSafe` is an
  audit marker. The two identical repo quoting helpers (`generation_read.rs:614`,
  `generations/mod.rs:1172`) are the only identifier escaping on the non-sea-query paths;
  `ddl.rs:280,394-401,450` interpolate `field.name()` inside `"..."` without doubling, safe only
  because model identifiers are validated (`model.rs:477`).
- **DataFusion unparser**: always quotes and doubles for `PostgreSqlDialect`; literals assume
  `standard_conforming_strings=on` (set at `generation_read.rs:608`). It could render (b) entirely
  and is the natural single-AST source for (e): build the policy selection as a `LogicalPlan`, run it
  in DataFusion, and `plan_to_sql` it for the PostgreSQL view. Caveats: unparsed SQL is
  syntactically, not semantically, equivalent; view text would change, which changes
  `physical_digest` (`ddl.rs:506`, it hashes the rendered lowering).
- **F4 trigger** (forward plan §7): "substantive typed variable joins/expressions beyond clear SQLx
  queries". Observed sites are static shapes with validated identifiers and almost no bound values;
  nothing found meets that trigger. The pinned crates already contain the F4 package (sea-query +
  `sqlx-utils` binder) without a new crate.

### 2.2 In-process relational compute (Q2)

- `lctx-model` may depend on arrow-* only. Available to it at the pin: `arrow-select`
  (take/filter/interleave/concat/zip), `arrow-row` (`RowConverter`; `Row`/`OwnedRow` are
  `Ord + Hash`, so they key hash or B-tree grouping and dedup), and, as a new direct dep with no new
  version, `arrow-ord` (sort, lexsort, partition, rank, comparisons). **Not available** without
  DataFusion: hash grouping (`GroupValues`), hash/sort-merge joins, window functions, recursive
  CTEs, UDAF machinery, MemTable.
- Contract mismatch: arrow-ord sorts are unstable; `memory.rs:order` promises tie stability and
  correctly uses a stable `sort_by` over arrow-row rows.
- DataFusion in `cpg-core` today is a provider/scan/admission conduit (`model_runtime.rs`,
  `prepared.rs`, `generation_read.rs`). Its joins, windows, recursive CTEs and UDFs are unused, while
  DESIGN B3 and ADR-0085 describe it as the relational compute layer. If the reviewer wants
  orchestration-side compute, candidates are joins that pair two decoded relations by id before a
  Rust kernel. Fit limits: DataFusion operators do not carry explicit-unknown/verdict semantics,
  charged-work refusal reasons or nominal `Id<T>` types; output order is undefined without `ORDER BY`
  (and `ORDER BY` uses DataFusion byte order, not PostgreSQL collation, as `generation_read.rs`
  already notes); hash joins cannot spill (disk manager disabled anyway).

### 2.3 Store boundary (Q3)

- Row decoding (`codec.rs`): no pinned library maps `PgRow` to a declared Arrow schema; the fork's
  `rows_to_arrow` does it for `tokio_postgres::Row` with inferred widths. pgpq has no decoder.
  serde_arrow fits only typed reads. Keep `codec.rs`; consider one decoder per driver (see table).
- COPY: pgpq + `copy_in_raw` already used; `copy_in_raw` is outside sqlx's SQL-safety check (C003),
  so `mod.rs:633`'s `format!` is the one unchecked statement on the write path (identifiers via
  `quoted`).
- Migrations vs generated DDL: sqlx `Migrator` owns the fixed service baseline only; generated
  generation DDL and the `verify.rs` shadow-install comparison have no pinned alternative.
  Observation on `MIGRATOR.run(&pool)` (lock left on a pooled connection after VersionMismatch, B024).
- Advisory locks: `PgAdvisoryLock` lacks shared and transaction scope; the raw SQL is necessary.
- LISTEN/NOTIFY: `PgListener` is available, with no consumer (F7).
- Cancellation: sqlx has none server-side; `statement_timeout` (already used) is the bound;
  tokio-postgres `cancel_token()` exists on the DataFusion scan path.

### 2.4 Concurrency, cancellation, budgets (Q4)

- `runtime.rs` uses tokio `Semaphore` owned permits, `timeout_at`, `spawn`/`spawn_blocking`, and a
  hand-rolled stop + counter + `Notify` drain. `tokio-util` 0.7.19 is already compiled:
  `CancellationToken` is usable with no feature change; `TaskTracker` needs feature `rt` (one
  feature on a crate already in the graph).
- DataFusion `MemoryPool` is already the substrate of `ResourceBudget` for compute
  (`model_runtime.rs:310-368`). Differences from attempt-budget charging: the pool counts only
  registered bytes; `charged.rs`/`batching.rs` add declared estimates (`NODE_ALLOWANCE`, row admission,
  poison-on-error) and named owners; the greedy pool has no fairness or priority; neither bounds CPU.
- Patch delta: tokio 1.53.2 (2026-10-03) fixes "sync: validate `MAX_PERMITS` in
  `Semaphore::acquire`", "task: drop replaced waker outside lock in `JoinSet`", "rt: drop blocking pool
  mutex before shutting down rejected task" and others; relevant primitives are in use. Pin-check route.

### 2.5 Contracts and observability (Q5)

- serde_arrow replaced the former hand-built `ArrowColumn` (generated codecs); postcard is in use.
- The remaining hand-written JSON Schema is `enum_schema!` (`serving/schema.rs`); schemars derive with
  `deny_unknown_fields` produces the same structure (`Proposed`, unverified byte equality).
- Tracing: one span site; `datafusion-tracing` 55.0.0 is compatible and lighter than previously
  recorded (no OpenTelemetry), but has little to instrument while DataFusion only scans.

### 2.6 Upgrade deltas (Q6, findings only; crates.io checked 2026-10-04)

| Crate | Pinned | Latest | Delta that matters |
|---|---|---|---|
| datafusion | 55.1.0 | 55.1.0 (2026-09-11) | none |
| arrow-* | 59.3.0 | 60.0.0 (released 2026-09-10 per CHANGELOG, crates.io 2026-09-15) | Requires a DataFusion release on Arrow 60 (none yet). Breaking: `Metadata` struct replaces `HashMap` metadata (touches schema construction in `lctx-model`), MSRV 1.88, sealed `ToByteSlice`. Features: rank for Utf8View/BinaryView, prefix-key caching in sort. Nothing that changes the Q2 conclusions. serde_arrow 0.15.1 already offers `arrow-60`. |
| sqlx | 0.9.0 | 0.9.0 | none |
| sea-query | 1.0.2 | 1.0.2 | none; `sea-query-sqlx` 0.9.1 (2026-05-30) is the external binder for sqlx 0.9 |
| pgpq / pgvector / serde_arrow | 0.12.0 / 0.4.2 / 0.15.1 | same | none |
| tokio | 1.53.1 | 1.53.2 (2026-10-03) | bug fixes in Semaphore/JoinSet/blocking pool (above) |
| tokio-util | 0.7.19 | 0.7.19 | none |
| datafusion-tracing | not linked | 55.0.0 (2026-08-24) | n/a |

## 3. Coordinator's contrary evidence (retired 2026-09-23 review, `b4ee5cc5`, §6–§8)

| Retired item | Reason then | Status at DataFusion 55.1 / current code |
|---|---|---|
| (a) Typed-plan rewrite of generated SQL rejected | SQL text was what `compiler_digest` hashed and snapshots tested; DataFusion shares no execution between queries | **Partly holds, different target.** Rule SQL no longer exists; DataFusion SQL is now scans plus one policy selection. Rendered **DDL** text is hashed (`physical_digest`, `ddl.rs:506`), so any rendering change (sea-query, unparser) changes digests: a rebuild under ADR-0078, not a blocker. Still true at 55.1 that DataFusion shares no execution across queries. For the ~20 scans, `ctx.table()` has no digest or snapshot impact. |
| (b) Recursive CTEs rejected | `RecursiveQueryExec` had no recursion limit | **Holds**: still "no limit or checks" (`recursive_query.rs:64`). Narrowed: `UNION` distinct now deduplicates across iterations and charges a memory reservation, so a finite-domain distinct recursion terminates or fails `ResourcesExhausted` under the pool; `UNION ALL` can still loop until the pool or the deadline. No current consumer. |
| (c) DataFusion `Constraints` rejected | Informational only | **Holds**, and moot: PostgreSQL CHECK/FK constraints now enforce (`ddl.rs`). |
| `FairSpillPool` deferred | `HashJoinInput` cannot spill | **Holds**: hash join still has no spill path at 55.1; spilling is disabled anyway (`DiskManagerMode::Disabled`, `model_runtime.rs:63`). No DataFusion joins run today. |
| `datafusion-tracing` deferred | Brings the tracing/OpenTelemetry 0.31 family | **Reason does not hold at 55.0.0**: no OpenTelemetry runtime dependency; adds `delegate` and `tracing-futures` only. The deferral can stand on "no consumer" (one span site, scans only). |
| `ArrowColumn` kept over serde_arrow | serde_arrow gates every Arrow bump | **Superseded**: serde_arrow 0.15.1 adopted (ADR-0085); `ArrowColumn` gone. The Arrow-bump coupling remains real (60 needs `arrow-60`, available). |

## 4. Uncertainties and absences

- Search coverage: `rg`/`grep` over `crates/*/src` (production) and `crates/*/tests` (counts only),
  `Cargo.toml`, `Cargo.lock`, `crates/lctx-workspace-hack/Cargo.toml`; pinned sources named above.
  Concurrent commits after `948b2a88` were not tracked.
- "No production DataFusion joins": searched for DataFrame join/aggregate calls and SQL text in
  `cpg-core/src`, `lctx/src`; SQL passed in at run time (`lctx query`, `GenerationReader::query`
  callers in tests) can be arbitrary and is out of this statement.
- `sqlx-utils` macros, `raw_query!`, and the schemars-derive equivalence were read from source, not
  compiled here (`not_run`).
- Whether the fork's pool exposes `tokio_postgres::Client::cancel_token` was not checked.
- sea-query absence of GRANT/SCHEMA/VIEW builders: grep of `sea-query-1.0.2/src` for grant,
  schema-create and view statements; extension modules (`extension/postgres`) hold types, functions,
  `ltree`, interval only.
- `TaskTracker` fit to `RequestExecution` grants is structural reading, not a line-by-line proof.
- Release notes: tokio from the GitHub release page; Arrow 60 from the arrow-rs CHANGELOG
  (summarised by a fetch tool, so treat feature bullets as indicative).
