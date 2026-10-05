# A3 — SurrealDB 3.3 capabilities against library-context obligations

Lane A3, Phase A evidence for the SurrealDB adoption design review. Researched 2026-10-05.
This lane reports capabilities, contracts and limits. It does not decide adoption.

## Baseline and sources

- **Version.** surrealdb / surrealdb-core / surrealdb-mcp 3.3.0. The crate source was read from the skill's cargo cache:
  `.claude/skills/neo4j-surrealdb/surrealdb/build/.cache/cargo-home/registry/src/index.crates.io-1949cf8c6b5b557f/`
  (surrealdb-core, -catalog, -expr, -syn, -sql, -kvs-*, surrealkv 0.21.4, surrealism-runtime 0.6.0).
  The GitHub tag `v3.3.0` was created 2026-09-24 and published 2026-09-28. The GitHub release body is empty; the release notes come from surrealdb.com/releases/3.3.0.
- **Skill.** `neo4j-surrealdb`, the SurrealDB half. Probe ids `SB###`/`SX###` come from its single 2026-10-05 run of 3.3.0.
- **Docs.** The skill's docs corpus (`surrealdb/content/corpus/docs`) is docs `main` at a pinned commit, plus the 3.2→3.3 migration guide.
- **Context7.** `/surrealdb/surrealdb.py` was used for Python SDK access only.
- **GitHub.** surrealdb/surrealdb issue search (semantic; coverage is partial) and the release list.
- **Scratch checks.** New scratch checks are labelled **A3-run**. They ran against the pinned image `surrealdb/surrealdb@sha256:681c6c22…cd20` (`/version` = `surrealdb-3.3.0`), `start --unauthenticated memory`, over HTTP `/sql`, on 2026-10-05. The scripts are in this scratchpad (`q.sh`, `ddl.surql`, `bulk.py`) and the container was removed afterwards. These are quick checks on the mem engine only. They are not skill probes and not performance claims.

Evidence labels: **[doc]** documented guarantee or description · **[src]** source-inspected at 3.3.0 · **[probe]** skill
probe-verified (`SB###`, 2026-10-05) · **[A3-run]** scratch check, mem engine, 2026-10-05 · **[issue]** GitHub issue
report · **[vendor-perf]** vendor performance claim · **[3rd-party]** non-vendor report.

---

## 1. Graph computation in the database

### Mechanisms
- **Edges.**
  - **RELATE edge tables.** Edges are records in their own table [probe SB020–SB027]. `TYPE RELATION IN a OUT b ENFORCED` refuses dangling endpoints (SB022); without ENFORCED they dangle. A pair gets duplicate edges unless a UNIQUE(in,out) index exists (SB020). `INSERT RELATION` bulk-loads edges (SB027).
  - **LIGHTWEIGHT (3.3).** The edge has no record, only its two vertex-side adjacency keys. It is synthesised on read with the canonical id `[in, out]`, so RELATE is idempotent per pair. It implies ENFORCED. It refuses fields, indexes, events, CHANGEFEED, `AS` views and LIVE, and `REMOVE TABLE` is refused while edges exist [doc define/table.mdx §LIGHTWEIGHT; src surrealdb-expr `table_type.rs:120-127`]. The A3-run test graphs used it successfully.
  - **INLINE field (3.3).** It copies a top-level, non-computed edge field into the adjacency value, so a traversal predicate evaluates without fetching the edge. It is not available on LIGHTWEIGHT relations. Payloads carry a field-set generation and fall back to the record when stale; they never evaluate silently [src surrealdb-catalog `schema/field.rs:104-111`, `table.rs` inline-field generation].
  - **INLINE EDGES n / INLINE REFERENCES n (3.3).** A per-vertex adjacency cache with a cap from 0 to 256. Each edge write rewrites the vertex's whole cache value. Range traversals, `VERSION` queries and INLINE-field filters bypass the cache [doc table.mdx l.867-892].
  - **Packed adjacency blocks (3.3).** A background "fold" packs adjacency keys into blocks (`--graph-fold-interval`, default 5 s; `SURREAL_GRAPH_FOLD_BATCH`). Numeric doc-id block encoding stays default-off "until removal epochs land", because `REMOVE TABLE` followed by re-creation can alias stale folded entries to new records [src surrealdb-catalog `table.rs` `graph_doc_ids` comment]. That is an acknowledged correctness hazard, gated off by default.
- **Record links and REFERENCE.**
  - A plain link outlives its target and reads NONE (SB023).
  - `REFERENCE ON DELETE IGNORE|UNSET|CASCADE|REJECT|THEN` handles deletion and lets `<~` read the link backwards [doc record-references.mdx; probe SB023/SB024].
  - **Writes are not checked:** `TYPE record<author>`, even with `REFERENCE ON DELETE REJECT`, accepted `author:ghost` on CREATE [A3-run t1]. Existence on write needs `ASSERT record::exists($value)`, which did refuse [A3-run t1]. That ASSERT is skipped under `OPTION IMPORT` (below).
- **Recursive idioms.** `.{n}`, `.{m..n}`, `.{..}`, `+collect`, `+path`, `+shortest=rec`, `+inclusive` and the `.@` repeat-recurse tree form [doc idioms.mdx l.986-1750; probe SB025].
  - **Depth.** Every bound is 1 to 256. `idiom_recursion_limit` is fixed at 256 and has no config key, despite the old env var [src `exec/config.rs:36-39`; `config.rs:108`; doc env vars "fixed limit of 256"].
  - **Truncation visibility.**
    - An *unbounded* `{..}` whose frontier is non-empty at 256 is a hard error ("Exceeded the idiom recursion limit of 256") [src `recursion/{collect,path,default}.rs`; A3-run: a 300-node chain with `{..+collect}` errors].
    - A *bounded* `{..256}` truncates silently at the bound and returned 256 rows with no signal [A3-run].
    - `+shortest` to a node more than 256 hops away errors [A3-run].
  - **Cycles.** `+collect` and `+shortest` keep visited sets and terminate on cycles. `+path` and Default walks carry **no visited set**: on a cycle they walk to the bound (upstream test `a_cycle_is_walked_to_the_bound_because_walks_carry_no_visited_set`) [src `recursion/path.rs:310`]. A3-run confirmed it: `{..4+path}` returned the walk `[b,c,a,b]`. Path enumeration is exponential in branching and bounded only by memory.
  - **Execution.** The operators are fully iterative BFS, so there is no stack risk. Values are RecordIds only; a non-record value ends the branch [src `recursion.rs` header].
  - **Third-party claim.** ArcadeDB's benchmark notes (v2.6.4, March 2026) report that `{1..30}` did not compose multi-hop paths [3rd-party, competitor, 2.x]. That is consistent with the documented behaviour that a range without `+collect` returns only the deepest level reached [doc idioms.mdx l.999], not with a defect. SB025 shows `+collect` reaches every hop at 3.3.
- **Fixpoint, worklist, SCC and accumulation.** There is no native fixpoint construct.
  - SurrealQL has `FOR`, `IF`, `LET`, `BREAK`, `CONTINUE`, but no `WHILE`. Outer variables **cannot be reassigned inside FOR** ("assignment operators are only allowed in SET…"); `array::fold` / `array::reduce` are the workaround [doc statements/for.mdx l.87-125].
  - A `DEFINE FUNCTION` body runs as one transaction and can recurse. Depth is capped by `max_computation_depth` (default 120, `SURREAL_MAX_COMPUTATION_DEPTH`): `fn::down(50)` worked and `fn::down(200)` failed with "Reached excessive computation depth" [doc function.mdx l.203-240; env vars; A3-run].
  - **SCC is expressible** as forward-reachable ∩ backward-reachable per node: `array::intersect(@.{..+collect+inclusive}->e->v, @.{..+collect+inclusive}<-e<-v)`. On a 5-node graph with one 3-cycle it gave correct components [A3-run t3]. Cost is O(V·(V+E)); it fails for components deeper than 256 hops. It is a demonstration, not a scalable SCC.
  - **Transitive closure** within 256 hops: `+collect` per source.
  - **Accumulation along paths.** `+path` returns vertex lists; `array::fold` over each path computes simple aggregates [A3-run]. Edge properties along paths need the `.@` tree form (`{ id, out_edges: ->e.{ cond, to: out.@ } }`), which returns nested objects [A3-run]. There is no semiring or path-algebra aggregation, and nothing like a BDD combine.
- **Graph algorithms.**
  - **None built in.** The skill catalogs 428 function paths from 3.3's parser table and has no `graph::` family. The top families are math, array, type, time, string, set, … [skill `content/index/functions.tsv`].
  - SurrealDB's own blog (2026-06-05): "For extremely graph-heavy workloads with complex graph algorithms (PageRank, community detection at scale), a dedicated graph database may still be preferable" [vendor].
  - No roadmap item was found. GitHub issue search for graph-algorithm feature requests returned 0 results [issue search, semantic, partial coverage]. That is not proof of absence.
- **Surrealism (WASM extensions).**
  - **Status.** Experimental: it needs `--allow-experimental surrealism` [doc capabilities.mdx l.111-114]. `DEFINE MODULE mod::x FROM f"bucket:/x.surli" UNSIGNED` (3.3 syntax). Silo packages are fetched over HTTPS. Signing is not implemented, hence the mandatory `UNSIGNED` [doc define/module.mdx; src `core/src/surrealism/CLAUDE.md`].
  - **Runtime.** wasmtime plus WASI (surrealism-runtime 0.6.0).
  - **Host API.** `sql(query, vars)`, `run(fn, version, args)`, a per-module in-memory KV, and stdout/stderr [src `surrealism-runtime/src/host.rs:24-40`].
  - **Transactional access.** `sql()` evaluates through `expr_compute` against a context derived from the calling query's `FrozenContext`, so it runs inside the caller's transaction, with capabilities narrowed to the module's declaration [src `core/src/surrealism/host.rs:146-171`]. That is transactional in principle, but every graph read is a host call that parses SurrealQL and marshals values across the WASM boundary.
  - **Limits.** `surrealism_max_memory`, `max_execution_time`, `max_kv_entries` (all None by default), `max_fs_bytes` 100 MiB, pool 8 [src `surrealism/config.rs`].
  - **Determinism.** Module state (linear memory, KV) **persists between invocations** and is per database [doc module.mdx "A module keeps state between invocations"]. Determinism is the module author's responsibility.
  - Probing: the skill did not probe Surrealism. Maturity: an experimental flag and an unsigned-only module path.

### Fit
Bounded reachability, k-hop, shortest unweighted path, cycle-aware collection and adjacency counts fit well, and 3.3 made them materially faster [vendor-perf].

Iterative or global analytics fit poorly or not at all:
- Leiden, PageRank, FCA, SCC scheduling over the whole graph, kNN graph construction, worklist fixpoints.
- Per-path condition combination over BDD atoms.

These need the client (Rust, as today) or Surrealism WASM. Surrealism is experimental and pays a host call per data access.

## 2. Derived data inside the database

- **COMPUTED fields.**
  - Evaluated on read. 3.3 refuses direct writes in the body; a write reached through a function fails on read [doc 32-to-33.mdx §COMPUTED].
  - Field clauses (DEFAULT/VALUE/COMPUTED) now run in dependency order, and ASSERT runs in a second pass [doc 32-to-33.mdx l.121-123].
  - 3.0.5 fixed eager evaluation of unselected computed fields [vendor release notes].
- **Table views `DEFINE TABLE … AS SELECT … [GROUP BY]`.**
  - Event-driven, incrementally maintained, materialised. Defining one runs the query over existing rows [doc table.mdx l.441-507].
  - **Limits:**
    - Only the `FROM` table triggers updates, so joined or linked tables go stale.
    - Views are read-only.
    - Writes under `OPTION IMPORT` (i.e., `surreal import`) **do not trigger views**; rebuild with `DEFINE TABLE OVERWRITE`.
    - The initial run is a full SELECT.
  - Past issue #5047 "DEFINE TABLE (view) exponentially slow performance at scale" is closed [issue].
- **Events.**
  - `DEFINE EVENT` is synchronous in the writing transaction by default.
  - `ASYNC [RETRY n] [MAXDEPTH 0..16]` runs it in a separate later transaction. It is enqueued atomically with the change and polled every 5 s by default [doc event.mdx l.413-505].
- **Change feeds and LIVE.** `CHANGEFEED <dur> [INCLUDE ORIGINAL]` with `SHOW CHANGES` (empty, with no error, on a table without one: SB043). LIVE SELECT needs ws or embedded and fails over http (SB041, SB051). Neither is allowed on LIGHTWEIGHT tables.
- **Generated from external declarations.** All of the above is SurrealQL DDL text, so a Rust model can emit it, as `lctx-model` emits PostgreSQL DDL today. `INFO FOR DB/TABLE` returns the canonical rendered DDL for snapshotting. There is no SurrealDB-side schema-as-code library beyond DDL (surrealkit and refinery are not covered by the skill).
- **Determinism and failure semantics.**
  - A failed synchronous event or ASSERT rolls back the statement or the BEGIN block (SB031).
  - `db.query()` returns `Ok` when an inner statement failed, so `.check()` is mandatory (SB013).
  - Async events only retry as configured.
  - Views can silently diverge after import or from non-FROM tables. That conflicts with "derived data must be exactly rebuildable", unless views are always rebuilt (OVERWRITE) before publication.

## 3. Schema and integrity at scale

- **Schema enforcement.**
  - SCHEMAFULL refuses undefined fields (SB028).
  - `DEFINE DATABASE … STRICT` refuses undefined tables (SB029).
  - A non-optional TYPE makes a field required (SB047).
  - `TYPE int` coerces 1.0 to 1 and refuses 1.5 and '1' (SB011).
  - `DEFINE INDEX … UNIQUE` over existing duplicates fails, and the index is then absent (SB030).
  - Composite UNIQUE indexes (`FIELDS a, b UNIQUE`) are accepted [A3-run DDL].
- **Composite content-derived keys.**
  - Array or object record ids, e.g. `t:[digest, 'k']` [doc; A3-run inserted 100k rows with `id: [i, "k{i}"]`].
  - Typed links `record<t>` can point at them.
  - **Correctness history:** HNSW and DiskANN built by 3.1.0-beta.1 through 3.2.4 over ids holding a number inside an array or object silently dropped records from kNN. Ids differing only in a nested number's type (`c:[1]` vs `c:[1dec]`) still collide in standard, HNSW and DiskANN indexes in 3.3 ("one of the two stays missing … kNN can return the other twice") [doc 32-to-33.mdx l.75-85]. Content-derived ids should therefore normalise nested numeric types (e.g., digests as strings or bytes).
- **Foreign keys.** These are not relational FKs.
  - A link field is one record id, so composite FKs become array-id links.
  - Existence on write needs `ASSERT record::exists($value)`. REFERENCE only governs delete behaviour [A3-run t1].
  - Edge tables check their endpoints with ENFORCED. ENFORCED is deferred during import [doc table.mdx l.797].
  - Cyclic or self references are not a structural problem; record links are just values.
  - There is no deferred-constraint mode except `OPTION IMPORT`, which **skips** ASSERT, field processing, events and views rather than deferring them [doc import.mdx l.143-147; field.mdx l.1195; A3-run: an ASSERT-violating CREATE succeeded under `OPTION IMPORT`]. Validation over stored contents must be a separate query pass. That matches the current "validate stored contents before publication" pattern, but it is bespoke code.
- **Codebook membership.** `ASSERT $value IN [...]`, a typed literal union, or a link to a codebook table plus `record::exists`.
- **CHECKs.** `ASSERT` expressions.
- **About 1,000 tables.** A3-run defined 1,100 SCHEMAFULL tables, each with 10 typed fields, one composite UNIQUE index and one REFERENCE field: 14,300 DDL statements, all OK in about 0.5 s on mem, and `INFO FOR DB` returned in about 18 ms. There is no evidence for RocksDB or SurrealKV at that DDL scale, and no evidence about planner or catalog-cache behaviour (`SURREAL_DATASTORE_CACHE_SIZE` = cached definitions) with 1k+ tables. GitHub search found no open large-schema issue [issue search, partial].
- **Value fidelity.**
  - u64 above i64::MAX wraps silently (SB046).
  - `serde_json` u64::MAX becomes a float.
  - NaN differs on round trip.
  - `-0.0` differs over ws and http [skill values catalog].
  - Bytes, decimal (`0.1dec`), nanosecond datetimes and uuids round-trip.
  - `Vec<f32>` widens to f64 arrays. The values are exact, but storage doubles unless HNSW `TYPE F32` is used.
  - NONE and NULL are distinct.
  - Values nested deeper than 256 levels are refused (3.3).
  - `vector<…>` is not a field type, so the dimension comes from `ASSERT array::len` or the HNSW `DIMENSION` (SB039).

## 4. Immutable snapshot/generation contract

- **Namespace or database per generation.** This is natural: `DEFINE DATABASE gen_<id> STRICT`, then load, validate, then expose. A3-run `REMOVE DATABASE` of a 100k-row database took about 0.01 s on mem; the cost on RocksDB is unmeasured.
  - **Issue reports:**
    - #7576 (open, 2026-10-03): RocksDB freezes permanently after a large delete transaction.
    - #7565 (open): RocksDB abort during a DELETE transaction in 3.3.0.

  Retirement by drop on RocksDB needs a probe.
- **Immutability enforcement.**
  - `DEFINE USER … ROLES VIEWER` at root, namespace or database level gives read-only system users [doc define/user.mdx l.68-72].
  - Table `PERMISSIONS` gives row-level control to record users.
  - There is **no per-database "read-only/frozen" flag** and no privilege-revocation equivalent of PostgreSQL's owner-enforced immutability: an OWNER or EDITOR can still write a published database. Immutability is a convention plus credentials. No search found a frozen flag; the search covered the `define/*.mdx` docs and `kvs` read-only grep.
- **Atomic publication.**
  - There is no publication primitive. The pattern is: load into a fresh database, validate, then flip a pointer record (e.g., `catalog:current = gen_id`) in one transaction. Readers resolve the pointer, then `USE DB`.
  - Pinning: readers can hold a ws session on a database. There are no leases, so retirement must check a reader-registration table in application code, as today.
  - Transactions are snapshot isolation with conflict-on-commit, uniform across backends, with no SERIALIZABLE level. `SELECT … FOR UPDATE` (3.3) covers named records only [doc transactions.mdx l.137-175].
- **Transaction size.**
  - `SURREAL_TRANSACTION_MAX_WRITE_KEYS` defaults to 0, i.e. unbounded [doc env vars l.549-552].
  - The write set is buffered in memory: RocksDB `OptimisticTransactionDB` [src `kvs-rocksdb/owned_tx.rs`].
  - Loading one generation in one transaction is therefore not credible at 11M rows plus index keys. Load in batches into an unpublished database and publish by pointer flip.
  - Payload limits: HTTP `/sql` rejected a 10k-row (~2 MB) JSON batch with **413** [A3-run]; WebSocket and gRPC limits are configurable.
- **Time travel.** `SELECT … VERSION <datetime>` works only on SurrealKV or RocksDB opened `versioned=true` (SB002/SB040; mem removed in 3.3). A non-existent timestamp returns an empty array [doc select.mdx l.1120], silently. This is not a substitute for named immutable generations.
- **Export and import.**
  - `surreal export` and `import` use SurrealQL text only. `--only` selects definition and record classes.
  - Import requires the `OPTION IMPORT` line, which skips events, views and field processing (3.0.4+) [doc export.mdx, import.mdx].
  - There is no binary, Arrow or Parquet snapshot. RocksDB checkpoints exist below SurrealDB, but nothing exposed was found.
- **Read-only open of an embedded store.**
  - **Not supported.** One RocksDB directory opens once (SB003). SurrealKV holds an OS file lock via `fs2` ("prevents multiple processes from accessing the same database") [src `surrealkv-0.21.4/src/lockfile.rs:15-28`].
  - No read-only or secondary open mode is exposed. The search grepped `kvs-rocksdb` and `kvs-surrealkv` for secondary/read_only open and found only read-only *transactions*.

## 5. Bulk ingest

- **Paths.**
  - `INSERT INTO t [..]` arrays (all-or-nothing per statement, SB016; `ON DUPLICATE KEY UPDATE`, SB017).
  - `INSERT RELATION` (SB027).
  - `surreal import` / `POST /import` / SDK `import()` / core `Datastore::import_stream`, all SurrealQL text.
  - Embedded `Surreal::insert`.
  - Surreal Sync (separate tool, "not yet stable") supports PostgreSQL via triggers, wal2json or pgoutput, full plus CDC [GitHub surreal-sync README].
- **No Arrow, Parquet or CSV loader** in the server or core. Core's Cargo.toml has no arrow, parquet or csv dependency, and a grep of core/SDK/expr sources was empty. CSV import exists only in Studio (GUI) [doc bulk-operations l.52-58]. From Arrow 59 this means serialising to `SurrealValue` or SurrealQL per row.
- **Throughput.**
  - No vendor bulk-ingest figure was found in the docs or the 3.3 notes. The 3.3 notes do claim 2.3–2.6x faster full-text indexed writes and about 2x faster restores of heavily indexed tables [vendor-perf].
  - A3-run: about 6.3k rows/s over HTTP into mem. That covered a schemaless table and a 10-field SCHEMAFULL table with a composite UNIQUE index, 2k-row batches, single client, and was dominated by Python JSON generation. **Not a performance claim**; it only shows that HTTP/JSON with one client is far from 11M-row-scale ingest.
  - Third party: ArcadeDB on v2.6.4 loaded 34M edges in 30 min over HTTP with a 1 MB payload limit [3rd-party, 2.x, competitor].
- **Issue reports.**
  - #7536 (open, 2026-09-24): RocksDB serial inserts stop responding at a fixed row count (307k on 3.3.0-beta.4); SurrealKV unaffected.
  - #7424 (open): RocksDB RSS about 9x the on-disk size, OOM under sustained ingest (3.2.0).
  - #7430 (open): SurrealKV 3.2.1 RSS 27.6 GB on a 547 MB dataset (block cache uncapped, ignores cgroup).
  - #7223 (closed): SurrealKV 100% CPU after bulk import.

## 6. Search

- **Vectors.**
  - Indexes are HNSW (`DIST`, `TYPE`, `EFC`, `M`) and DISKANN. MTREE is gone (SB037–SB039).
  - `<|k,ef|>` without an HNSW index returns [] silently (SB038).
  - `<|k,COSINE|>` is an exact brute-force scan (SB037).
  - **Pre-filtered kNN (3.3).** Index-answerable WHERE predicates become an allow-list, applied in tiers [doc similarity-search.mdx l.160-206]:

    | Allow-list size | Tier | What the search does |
    |---|---|---|
    | ≤2,000 | `exact` | exact distances |
    | ≤100,000 | `graph` | ef×4 |
    | >100,000 | `graph_unboosted` | the query's ef |
    | — | `fallback` | used when `SURREAL_BITMAP_BRANCH_BUDGET` is exceeded |

    Thresholds are set by env vars. `WITH` or `VERSION` queries are not pre-filtered. Only indexes built on 3.3 participate.
  - `SURREAL_HNSW_CACHE_SIZE` bounds vector caching, but the HNSW adjacency graph stays resident outside that budget [doc env vars l.118-121].
  - `SURREAL_HNSW_BUILD_SEED` exists for reproducible builds and is documented as not for production. Exactness control is index-free brute force or the `exact` pre-filter tier; there is no recall measurement.
- **Full text.** `DEFINE ANALYZER` plus `FULLTEXT ANALYZER … BM25(k1,b) [HIGHLIGHTS]`, queried with `@n@`, `search::score` and `search::highlight` (SB035). `SEGMENT` (CJK) needs the `cjk` feature when embedded (SB036). In 3.3, postings are keyed by document [vendor].
- **Hybrid.** `search::rrf([...], k, 60)` and `search::linear` exist in 3.3's function table [skill functions.tsv; doc hybrid-search.mdx l.51-106]. Not probed by the skill.
- **Comparison with current pgvector.** Current retrieval keeps exact float32 values canonical. SurrealDB brute force gives exact results; HNSW is approximate with unmeasured recall.

## 7. Deployment and access

- **Engines.** Embedded mem, SurrealKV and RocksDB, single process per directory (SB003; SurrealKV fs2 lock). A server provides ws/http/grpc with identical answers (SB056), Postgres wire (3.3; jsonb rows, SurrealQL not SQL: SB080/SB081), GraphQL, `/gql`, and MCP (14 tools; `surreal mcp --log none`: SB086/SB087).
- **Several processes reading one store** (a compiler plus several MCP processes) **requires the server.** Embedded is single-process. Alternatively one process embeds and the others connect to it over ws.
- **Python.**
  - The `surrealdb` PyPI package is at 2.0.0 (2026-04-23, requires Python 3.10+). It has ws, http and embedded (`surrealdb[embedded]`: memory, file, surrealkv, surrealkv+versioned) [Context7 `/surrealdb/surrealdb.py`; PyPI JSON].
  - Its compatibility with server 3.3 is not established here.
  - The skill does not index it. The repository's FastMCP adapter reaches a Rust native layer, so the Rust SDK through pyo3 is the natural route.
- **Errors and retries.**
  - Conflicts surface at commit, and the SDK does not retry. On mem the error is kind `Internal` with only a message; over ws it is `QueryError::TransactionConflict` (SB033/SB053).
  - A UNIQUE violation is not `is_already_exists()` (SB015).
  - A parse error fails the whole call (SB049).
  - `query()` returns Ok with failed statements inside (SB013).
- **Builds.** Rust 1.95 floor (SX001); the workspace toolchain (nightly-2026-09-29) satisfies it. The RocksDB engine needs a C++ toolchain. surrealdb-core is "unstable, not SemVer, changes between patch versions" [probe c-unstable; doc 32-to-33.mdx §Rust lists Datastore API changes in a minor release].
- **Licence.** BSL 1.1 until 2030-01-01 (Apache-2.0 afterwards). It restricts offering a "Database Service" to third parties [skill NOTICE]. Not a technical criterion; recorded for completeness.

## 8. Hybrid patterns with PostgreSQL

| Pattern | Mechanism | What it buys | Cost / risk |
|---|---|---|---|
| H1: SurrealDB as a rebuildable graph/serving projection of a published PG generation | After PG publication, a Rust exporter reads the generation (DataFusion or sqlx) and writes LIGHTWEIGHT or INLINE edges plus node records into SurrealDB database `gen_<id>`, then flips a pointer. B10 already allows "a rebuildable search projection" and §6.4 forbids a second canonical store, so this would be a derived artifact keyed by generation identity | Native k-hop, `+shortest`, `<~` back-links, filtered kNN, BM25 and RRF in one query language; MCP and GraphQL front-ends | A second store to operate (server process for multi-process readers); a bespoke exporter (no Arrow import); exactness of derived content must be proven (row counts and digests); no analytics gain (Leiden, PageRank, FCA and SCC stay in Rust); the projection is only as fresh as the generation |
| H2: SurrealDB fed by Surreal Sync CDC from PG | pgoutput logical replication | No exporter code | Surreal Sync is "not yet stable"; CDC over immutable, schema-per-generation tables is a poor match (publication is a schema, not row churn); mapping of FKs to links is undocumented |
| H3: SurrealDB primary, PG auxiliary (e.g., pgvector or analytics) | Inverse of H1 | Graph-native model | Loses PG's owner-enforced immutability, COPY bulk path, relational FKs, SERIALIZABLE and the mature tooling the current trust boundary depends on; all §6.1/§6.2 publication and lease semantics need re-implementing |
| H4: Embedded SurrealDB (mem engine) as an in-process graph query engine per serving process | Each MCP process loads its projection into `Mem` at start | No server; single-process lock is irrelevant | Load time and memory per process; mem has no VERSION; comparable to the existing petgraph snapshots, but offers query language rather than algorithms |

## 9. Maturity and risk

- **Release cadence.** About monthly minors in 2026: 3.0.x (Feb–Mar), 3.1.0 (2026-06-05), 3.2.0 (2026-07-06), 3.2.4 (2026-08-17), 3.3.0 (tagged 2026-09-24, published 2026-09-28). There were three or four betas per minor, and 2.6/2.7 and 3.1.6 backports continue [GitHub releases].
- **Migrations.** 3.3 applies irreversible datastore migrations at first start; rollback to 3.2 is by export and import only. Definitions written by 3.3 cannot be decoded by 3.2 [doc 32-to-33.mdx l.55-60, 99]. Upgrades are therefore one-way; pinned regeneration from inputs (the current model) sidesteps this.
- **Correctness history.**
  - Silent kNN omissions for array or object ids with numbers in 3.1–3.2.
  - Nested-number id collision is still present in standard, HNSW and DiskANN indexes in 3.3.
  - The graph doc-id aliasing hazard is gated off by default.
  - Sequence keyspace collision with tables named `sq*`, broken through 3.2.
  - Writing COMPUTED bodies wrote on every read in 3.2.4.
  - 18 security fixes in 3.3 [doc 32-to-33; vendor notes].
- **Open issue reports (storage engines).** #7576, #7565, #7536 (RocksDB, 3.3 or 3.3-beta), #7424, #7383, #7359 (RocksDB memory and wedging), #7430 (SurrealKV memory), #7426 (SurrealKV will not start after an unexpected shutdown), #6735 (embedded RocksDB: parameter binding causes silent data loss after schema init, open since 2026-01) [issue].
- **Graph traversal performance.**
  - The 3.3 adjacency rebuild is new code: blocks, INLINE, LIGHTWEIGHT.
  - Claims [vendor-perf]: "dramatically" faster traversals, and count(->edge) from adjacency.
  - The 3.3 notes contain no traversal benchmark numbers; the numbers they do give are 65 µs to first rows over gRPC and 2.3–2.6x full-text writes.
  - No independent 3.3 traversal benchmark was found. The ArcadeDB 2.x report predates the rewrite.

---

## Obligation → mechanism → fit

| # | Obligation | SurrealDB 3.3 mechanism | Fit | Key evidence |
|---|---|---|---|---|
| 1a | Bounded traversals, reachability, shortest path | `.{m..n}`, `+collect`, `+shortest`, `<~`, INLINE/LIGHTWEIGHT | **Good** within 256 hops; unweighted only | SB025, src recursion/*, A3-run |
| 1b | SCC scheduling, fixpoints, worklists | None native; per-node reachability intersection; fn recursion ≤120; no WHILE or mutable loop vars | **Poor** (demonstrable on tiny graphs, O(V·(V+E)), 256-hop cap) | A3-run t3, for.mdx, env vars |
| 1c | Leiden, PageRank, FCA, kNN graph | None built in; Surrealism WASM (experimental) or client | **None** in-database | functions.tsv, vendor blog |
| 1d | Per-path condition combination (BDD) | `+path` / `.@` trees + `array::fold`; BDD only via WASM | **Poor** | A3-run, B10 condition kernel |
| 2 | Derived relations from declarations | DDL text: COMPUTED, `AS SELECT` views, events, changefeeds | **Partial**: views stale on non-FROM tables and after import | table.mdx, event.mdx |
| 3a | Typed closed schema, ~1,100 tables | SCHEMAFULL + STRICT + typed fields | **Good** (1,100 tables OK on mem) | SB028/029/047, A3-run |
| 3b | ~6k FKs incl. composite/cyclic | Array-id links + `ASSERT record::exists`; REFERENCE delete rules; ENFORCED edges | **Partial**: no write-time check by type; skipped under import | A3-run t1 |
| 3c | Uniques, CHECKs, codebooks | UNIQUE (composite), ASSERT | **Good**, with SB030/SB048 traps | probes |
| 3d | Value fidelity | Typed values | **Partial**: u64 wrap, -0.0 over wire, nested-number id collisions | SB046, values catalog, 32-to-33 |
| 4a | Immutable published generations | Database per generation + VIEWER users | **Weak**: no frozen database; convention only | user.mdx |
| 4b | Atomic publish after validation | Batch-load into a hidden database, validate, pointer flip | **Feasible, bespoke** | transactions.mdx |
| 4c | Reader leases, retirement | Sessions; REMOVE DATABASE | **Bespoke leases**; drop cost on RocksDB unknown (#7576) | A3-run, issues |
| 5 | Bulk ingest of 11M rows | INSERT batches / SurrealQL import; no Arrow/CSV | **Weak / unmeasured** | src grep, A3-run 413, issues |
| 6a | Exact 1024-d f32 retrieval | Brute force `<|k,DIST|>`; HNSW/DiskANN approximate; pre-filter tiers | **Partial** (exact = scan) | SB037/038, similarity-search.mdx |
| 6b | BM25 + hybrid | FULLTEXT BM25, `search::rrf` / `search::linear` | **Good** (hybrid not probed) | SB035, functions.tsv |
| 7 | Compiler + several MCP readers | Server (ws/grpc); embedded is single-process | **Server required** | SB003, surrealkv lockfile.rs |
| 8 | Hybrid with PG | H1 rebuildable projection most coherent | **Credible as an additive projection** | §8 table |
| 9 | Maturity | Monthly minors, unstable core, open storage bugs | **Elevated risk** | issues, 32-to-33 |

## Traps (consolidated)

1. A bounded recursion `{..N}` truncates silently at N; only unbounded `{..}` errors at 256. Depth is capped at 256 and cannot be configured [src, A3-run].
2. `+path` and Default recursion have no visited set, so cycles are re-walked to the bound and paths grow exponentially [src path.rs:310, A3-run].
3. `record<t>` and `REFERENCE` fields accept non-existent targets on write. Use `ASSERT record::exists($value)` [A3-run].
4. `OPTION IMPORT` (all `surreal import` and `/import`) skips ASSERT, field processing, events and views, and defers ENFORCED. CREATE returns `[]` under it [A3-run; doc].
5. Views update only from the `FROM` table and not on imports [doc].
6. Record ids that differ only in a nested number's type collide in standard, HNSW and DiskANN indexes even on 3.3 [doc 32-to-33].
7. HTTP `/sql` rejects payloads around 2 MB with 413 [A3-run].
8. `SURREAL_TRANSACTION_MAX_WRITE_KEYS` defaults to unbounded, and the write set is held in memory [doc, src].
9. A `VERSION` query at a non-existent timestamp returns `[]` silently; mem has no VERSION in 3.3 [doc].
10. `FOR` loops cannot reassign outer variables; function recursion is capped at 120 [doc, A3-run].
11. Surrealism modules keep state between invocations, are experimental and are unsigned-only [doc, src].
12. The HNSW adjacency graph sits in resident memory outside `SURREAL_HNSW_CACHE_SIZE` [doc].
13. Existing traps still apply: SB013 (query Ok with failures), SB046 (u64 wrap), SB038 (empty kNN without an index), SB020/SB022 (duplicate and dangling edges), SB033 (conflict typing on mem), SB003 (single process).
14. Upgrades past a minor are one-way datastore migrations [doc].

## Unresolved

- Ingest throughput and memory for about 11M rows across about 1,100 tables, embedded RocksDB or SurrealKV via the Rust SDK. Nothing has been measured; the open issues suggest risk (#7536, #7424, #7430).
- Traversal latency of 3.3 LIGHTWEIGHT and INLINE against the current petgraph snapshot at real scale.
- `REMOVE DATABASE` cost and stability on RocksDB at generation size (#7576).
- Whether a VIEWER system user plus no-OWNER credentials can be made equivalent to the current privilege-enforced immutability, or whether any form of frozen database exists. None was found; the search covered the define docs and the kvs source.
- Surrealism: per-host-call overhead and practicality of running biodivine-lib-bdd or a Leiden kernel inside wasmtime with `sql()` reads. Not probed.
- `search::rrf` and `search::linear` exact semantics, and DiskANN query behaviour. Not probed.
- Python SDK 2.0.0 compatibility with server 3.3.
- SurrealKV and RocksDB conflict behaviour (the skill probed mem only).
- Index-build time on large existing tables.

## Candidate Phase B probes

| ID | Question | Minimal setup | Control | Outcome that would change the judgment |
|---|---|---|---|---|
| P1 | Can embedded RocksDB or SurrealKV ingest a generation-sized load? | Rust SDK, embedded `rocksdb://` and `surrealkv://`; real `lctx-model` DDL for about 1,100 tables (generated SurrealQL); 11M rows from a PG generation via Arrow → `SurrealValue` `insert` batches of 1k–10k; RSS sampled | Same rows into mem; PG COPY time for the same generation | Under about 3x PG COPY time with bounded RSS would make H1 or primary plausible; stalls (#7536) or RSS ≫ data size would rule out primary and weaken H1 |
| P2 | Traversal performance against petgraph | Load the real dependency or call graph as LIGHTWEIGHT edges (+ INLINE EDGES 64); run representative k-hop, `+collect`, `+shortest` and back-reference queries | The same queries on the existing petgraph snapshot | SurrealDB within about 2x of petgraph would make it credible as a serving query engine; ≫10x would leave it a projection only for ad-hoc queries |
| P3 | Integrity under bulk load | Load with `OPTION IMPORT` and with plain INSERT; seed known FK, UNIQUE and ASSERT violations; run a validation pass (`record::exists`, count checks) | The PG generation's FK and CHECK rejection of the same seeds | Plain-INSERT validation cost acceptable and every seed caught would make the integrity mapping feasible; otherwise the validation pass is a bespoke subsystem |
| P4 | Generation publication and retirement | Database per generation, VIEWER user for readers, pointer-flip transaction, concurrent ws readers, then `REMOVE DATABASE` of a full-size generation on RocksDB | The same lifecycle on PG schemas | A freeze, wedge or long stall on remove (#7576) would rule out per-generation databases on RocksDB |
| P5 | Recursion-truncation visibility at scale | Graph with chains longer than 256 and cycles; `{..N+collect}`, `{..+collect}`, `{..N+path}` | Ground truth from petgraph | Confirms the silent-bound behaviour on persistent engines, and path blow-up memory |
| P6 | Exact and filtered vector retrieval | 1024-d f32 embeddings from a real spec; brute force `<|k,COSINE|>`, HNSW `TYPE F32`, pre-filtered queries across the 2k/100k tiers | pgvector exact results on the same vectors | Recall 1.0 for the exact tier with a stable top-k order would allow parity; recall loss would leave pgvector as the authority |
| P7 | Surrealism viability for in-database analytics | Small Rust WASM module (SCC via Tarjan, or a BDD combine via biodivine-lib-bdd compiled to wasm32-wasi) reading adjacency via `sql()`; `--allow-experimental surrealism` | The same algorithm natively in Rust over an exported adjacency list | Host-call overhead within about 5x would let WASM host graph kernels; otherwise analytics stay outside the database |
| P8 | Multi-process access | One server (SurrealKV) with one writer (compiler) and 3 reader processes over ws during a load | Embedded open from a second process (expected refusal) | Confirms the server-only topology and its conflict/retry behaviour on persistent engines |
| P9 | Hybrid fusion semantics | FULLTEXT BM25 + HNSW + `search::rrf` / `search::linear` on a doc subset | Manual RRF in Rust over the separately-ranked lists | Exact agreement would let hybrid search be delegated |
| P10 | Nested-number id collision with content-derived keys | Records with ids `[digest_bytes, 1]` versus `[digest_bytes, 1dec]` / `1f`; standard and HNSW indexes | The same ids as all-string arrays | Confirms the key-encoding rule (normalise to strings or bytes) needed for content-derived composite keys |

## Files written by this lane

- `/tmp/claude-1000/-home-paul-library-context/3307aa74-dd84-437f-93be-1d92a29d0cce/scratchpad/a3-surrealdb-capabilities.md` (this report)
- Scratch: `.../scratchpad/q.sh`, `.../scratchpad/ddl.surql`, `.../scratchpad/bulk.py`, `.../scratchpad/src/` (extracted crate sources)

No repository files, evidence folders or skill files were modified.
