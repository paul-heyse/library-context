# A1 — Store-coupled surface of library-context (evidence for SurrealDB/Neo4j design review)

Baseline: `main` at 1158ebe2 (2026-10-05), read-only. Uncommitted AGENTS.md / `.config/library-skills.toml` were not relevant.
Method: source reading plus grep/line counts (`src` = lines before the first `#[cfg(test)]`; integration tests are counted separately), read-only `psql` against the live `lctx` database (`SET default_transaction_read_only=on`), and `git log --since=2026-09-14`.
No product tests, compiles or `just` recipes were run.

## 0. Headline observations (facts; interpretation is marked)

1. **Half of `lctx-postgres` is serving hydration, not PostgreSQL code.** It has 21,509 src lines. 11,226 of them are serving services, and these contain almost no SQL. They reach data through one generic primitive, `GenerationLease::read_for/read_ids/visit_for`. That primitive issues `SELECT <declared cols> FROM g.rel WHERE fk = ANY($1) ORDER BY …` (`crates/lctx-postgres/src/generations/mod.rs:939-1006`, `1050-1143`). There are 252 such call sites, chained one hop at a time in Rust, for example member → invocation → seed → brief at `operation_sections.rs:1101,1151-1154` and `packet_service.rs:208-218`.
   - *Interpretation:* serving is multi-hop path hydration done as sequential round trips. This is the clearest graph-shaped workload in the repository.
2. **DataFusion does no relational compute in compilation.** All 19 compile-time DataFusion query sites in `cpg-core` are `SELECT * FROM "<relation>"`, used as a typed scan transport into Rust domain kernels (§2(h) lists the sites). The kernels hold the joins and graph logic in memory.
   - No JOIN, GROUP BY or recursive query exists in compute SQL. The only JOINs are the 5 model-declared CallPolicy views (`lctx-model/src/domain/normalized/events.rs:56-58`) and control/catalog SQL.
   - DESIGN §15.11 now says this explicitly ("Most current derivations and semantic validators invoke pure model kernels…", plus the CI-07 rationale). §15.10 still says "Relational questions stay SQL".
3. **Graph algorithms are a small, localised part of the model.** Code that uses petgraph, leiden-rs or fixedbitset is about 4.6k of `lctx-model`'s ~124k src lines (projection, analytics, delegation, derivation, summary schedule).
   - The largest model modules are typed fact semantics: `execution` 26.3k, `normalized` 14.2k, `synthesis` 7.0k, `selection` 5.8k, `catalog` 5.7k, `analysis` 5.5k.
   - Fixpoint-shaped work (summary worklists, BDD conditions) is domain-specific, not generic traversal.
4. **Integrity is model-authoritative; PostgreSQL re-enforces it.** `lctx-model::domain::memory::MemoryGeneration` (703 lines) validates "the same way" as PostgreSQL (unique keys, complete and subtype-correct references, ordered content digest, every model invariant), with no database. Invariants are Rust visitors streamed over Arrow frames from PostgreSQL in `validation_session.rs`. The DDL constraints are a generated second enforcement.
5. **The live generation is staging, not published.** `lctx_gd3a3fa026a1237a1c4e175b9b1019eeb` is `state=staging`, `frontier=catalog`, with 0 receipts, 9 stage outcomes (acquire … normalize_relations) and no advisory locks, so it is effectively *interrupted*. Consequences for the catalog facts:
   - Nominal-reference FKs, installed only at the `validated` phase, are absent.
   - 597 of 784 canonical tables are empty.
   - Counts below describe a partial generation. Constraint *shapes* are still representative because the DDL is uniform.

## 1. Responsibility map

Lines are src lines unless noted. "Generic" means a mechanism a database could plausibly provide. "Domain" means semantics that would remain whatever the store.

### (a) Generation lifecycle, publication, leases, roles, grants, receipts, audit — `lctx-postgres` ≈ 4,830 src + `control.sql` 182 + migrations 74 + `sql/database.sql` 24

| File | src | Substrate coupling | Contract | Generic vs domain |
|---|---|---|---|---|
| `generations/lifecycle.rs` | 907 | sqlx transactions, advisory locks, GRANT/REVOKE | attempt-owned generations, staging→sealed→validated→published, failed/interrupted (§15.11 Lifecycle; ADR-0086, ADR-0094) | Mostly generic immutable-snapshot plus MVCC machinery. The typed attempt states are domain. |
| `generations/receipts.rs` | 545 | 43 sqlx calls; `LOCK TABLE … ACCESS EXCLUSIVE`; SAVEPOINT trial install of FK DDL then rollback (`:374-381`) | seal, stored-content validation, publication, failure (§15.11) | Mixed: content digests and receipts are domain; lock and grant choreography is substrate. |
| `generations/lease.rs` | 327 | session-level shared advisory lock; driver-neutral (`LeaseDriver`) so both sqlx and tokio-postgres run it | readers pin a published generation and check model, physical and prefix digests (§15.11; P1.10) | Generic snapshot pinning. Digest binding is domain. |
| `generations/locks.rs` | 100 | `pg_advisory_*` | lock order installation → selection → generation → attempt → relations | Generic. |
| `generations/failure.rs` | 211 | renders a CHECK for failure classes | typed failure record | Domain taxonomy; storage is generic. |
| `generations/catalog.rs` | 209 | control-schema reads plus `pg_locks` | `lctx generation list|show` | Generic. |
| `generations/install.rs` | 177 | 36 sqlx calls; exclusive try-lock | `store install|reset` (P1.6) | Generic. |
| `generations/verify.rs` | 650 | `pg_class`/`pg_attribute`/`pg_constraint`/ACL introspection; shadow install in a rolled-back txn | `store check`: live catalog equals this binary's lowering, roles and privileges (P1.6) | Exists only because the schema is generated DDL in an external server. |
| `generations/audit.rs` | 522 (+79 tests) | 26 sqlx calls | explicit read-only recomputation of acknowledged frames (ADR-0126) | Domain proof reuse; storage-specific reads. |
| `generations/vocabulary.rs` | 533 | private `__delta_*` tables, `security_barrier` prefix views, SQL conflict/mutation checks (`:267,276`), FK trial in SAVEPOINT (`:292-324`) | immutable vocabulary epochs and publication groups (ADR-0105, ADR-0108; §15.11 P4) | Substrate realization of a domain rule (append-only prefix authority). Live schema: 302 delta tables and 84 prefix views. |
| `roles.rs`, `bootstrap.rs`, `lib.rs`, `connection_options.rs` | 166 / 31 / 471 / 66 | four PG roles, pools, sqlx migrations | provisioning (P1.5, WP0.6) | Generic. |
| `generations/control.sql`, `migrations/*.sql`, `sql/database.sql` | 182 / 74 / 24 | PG DDL | control schema (17 tables), `lctx_ops`, `lctx_cache` | Generic. |

### (b) Schema/DDL lowering and type mapping — `lctx-postgres` 728 + `lctx-model` record/model ≈ 1.3k + macros 930

- **`generations/ddl.rs` (599).** Uses sea-query `Table`/`ForeignKey`/`Index` plus `Expr::cust` CHECK strings (`:191-439`). Per relation it emits:
  - PK `(generation_id, id)`;
  - CHECK `generation_id = literal`;
  - CHECK `octet_length` for Id/Digest;
  - CHECK on code domains (`IN (…)`);
  - finite-float and negative-zero CHECKs;
  - array-shape CHECKs;
  - a row-wire-bytes CHECK (≤ `MAX_ROW_BYTES`);
  - sum-type arm CHECKs plus UNIQUE `(generation_id, id, tag)`;
  - nominal FKs `(generation_id, field[, tag]) → target(generation_id, id[, tag])`.

  It also emits derivation views, CallPolicy views, serving mapping views and lookup indexes (`:357-410`), and the grant/revoke phases. The physical digest covers all of it (`:440`).
- **`generations/physical_columns.rs` (128).** Maps Scalar to PG types: Id/Digest/Binary → `bytea`, FiniteF64 → `double precision`, lists → arrays. It adds hidden `generation_id` and `introduced_epoch` columns.
- **`lctx-model/src/domain/record.rs:8-30,125,464-520`.** Scalar → Arrow `DataType` (Id = `FixedSizeBinary(16)`) and the `Record` trait schema.
- **`model.rs:6,89`.** `Relation` schema.
- **`lctx-model-macros/src/lib.rs` (930).** `Domain` (528 uses), `DomainSum` (102) and `DomainCode` (221) derives generate serde_arrow encode/decode (`:474-490`, `:897-908`).
- Contract: "PostgreSQL is a lowering of the validated domain, never a public table specification" (`ddl.rs:1-3`; ADR-0085, ADR-0086; DESIGN §15.11).
- *Interpretation:* Arrow lowering is domain-adjacent and DB-agnostic, since DataFusion and MemoryGeneration also use it. Only `ddl.rs` and `physical_columns.rs` are PG-specific.

### (c) Load path — `generations/mod.rs` `copy_into` (`:583-711`) + `codec.rs` (88) + `lctx-model/domain/batching.rs` (193)

- pgpq `ArrowToPostgresBinaryEncoder` writes `COPY … FROM STDIN BINARY` through sqlx `copy_in_raw`.
- **Encoding is row by row** (`batch.slice(index,1)` per row, `:655-680`), with a per-row size admission against the budget and `MAX_ROW_BYTES`, and abort on refusal.
- The read side, `codec::decode`, maps sqlx rows to the declared Arrow schema. Type dispatch exists only at this boundary.
- Contract: §15.11 store; ADR-0086. Generic mechanism. The per-row admission is a domain resource policy (`domain::resources`).

### (d) Integrity and validation — three layers

1. **Database constraints, generated (live counts in §2).** They cover structural invariants only: keys, byte widths, code domains, sum arms and nominal FKs. FKs are installed at `validated` and trial-installed in SAVEPOINTs at checkpoints and vocabulary closes.
2. **Shared model validators (authoritative).** `lctx-model/domain/validation.rs` (1,137) declares invariants and publication checks (`invariants_for`, `publication_checks_for`, `:1125-1131`). `memory.rs` runs them without a database.
3. **Validation execution in PostgreSQL.**
   - `validation_session.rs` (945 src + 1,191 inline tests) streams frames via `visit_named` into model visitors and records proof receipts and frame digests. It has 81 sqlx calls, mostly for receipts and frames.
   - `stage_validation.rs` (305) handles consumer eligibility.
   - `publication_validation.rs` (152) executes model-owned publication checks.
   - `validation_views.rs` (52) handles model-declared earlier vocabulary views.

Which is authoritative: the model (layer 2) per `validation_session.rs:1-2` ("read grants refer to them, rather than constituting a second semantic authority") and `memory.rs:1-4`. Database constraints duplicate structural checks as defence in depth. Database-side "validation SQL" is limited to vocabulary conflict/mutation checks (`vocabulary.rs:267,276`) and reference-trial DDL.

### (e) Graph persistence and hydration (projection snapshots)

- No PG-specific code. Snapshots are ordinary relations (`projection_snapshots`, `projection_snapshot_chunks` and 22 other `*projection*` relations in the live schema), written through the generic COPY path.
- Encoding: `lctx-model/domain/projection/snapshot.rs` (399; Postcard).
- Builder: `projection/normalization.rs` (1,162), `projection.rs` (331), `native.rs` (203).
- Hydration: `cpg-core/src/analysis_graphs.rs` (227) via `snapshot::hydrate` (`:166`).
- Contract: DESIGN §15.10 and ADR-0103 / ADR-0106 ("rebuild unconditionally…; no separate graph schema … becomes semantic authority").
- *Interpretation:* a graph store would directly replace this mechanism (≈2.3k lines). The topology contracts (roles, universes, parallel-arc policy, canonical ordering) are domain and would remain.

### (f) Search, retrieval, vectors and embedding cache

- `generations/vectors.rs` (673). About half is `pg_catalog`/ACL self-verification (`:404-652`). Similarity is exact cosine via pgvector `OPERATOR(lctx_ext.<=>)` over `lctx_cache.serving_vectors` (`:354`), with no ANN index.
- `cache.rs` (178): `lctx_cache.embedding_values`/`specs`.
- `retrieval_service.rs` (984): ranking policy in Rust "around numerical library callbacks". It is domain.
- Lexical scoring is BM25S in Python (`python/lctx_mcp/src/lctx_mcp/retrieval.py`, 68) over a corpus Rust supplies.
- `cpg-core`: `embedding_realization.rs` (219 src), `retrieval_preparation.rs` (98), `analytic_embedding.rs` (270), `embedding_service.rs` (99). Plus `lctx-embed` (233) and `python/lctx_mcp/embedder.py` (199).
- Live state: the `lctx_cache` tables are empty (0 rows). The extension is `vector 0.8.6` in schema `lctx_ext`.
- Contract: synthesis-and-serving §11.1–11.2.
- Generic: vector storage and cosine. Domain: specs, ranking and fusion.

### (g) Serving queries, hydration, generated views and cursors — 11,226 src in `lctx-postgres` + `python/lctx_storage/src/serving.rs` 1,146 + `lctx-model/domain/serving` ≈ 4.3k

| File | src | SQL coupling |
|---|---|---|
| `operation_sections.rs` | 1,670 | 0 direct sqlx (8 `read_for`/`read_ids`) |
| `catalog_service.rs` | 1,258 | `visit_verified` |
| `packet_service.rs` | 1,001 | 6 lease reads |
| `retrieval_service.rs` | 984 | — |
| `capture_packets.rs` | 798 | — |
| `source_usage_service.rs` | 727 | — |
| `evidence_service.rs` | 683 | 3 sqlx (artifact bytes) |
| `packet_reads.rs` | 682 | relation-scope checks |
| `source_characterization_service.rs` | 651 | — |
| `capability_service.rs` | 622 | — |
| `flow_inventory_service.rs` | 478 | — |
| `selection.rs` | 469 | 12 sqlx (selection and admission) |
| `native_service.rs` | 365 | — |
| `runtime.rs` | 304 | CPU admission |
| `guard.rs` | 177 | — |
| `access_routes_service.rs` | 148 | — |
| `service.rs` | 83 | — |
| `serving_shape.rs` | 69 | view/index inspection |
| `reader.rs` | 60 | — |

- Generated views: `ddl.rs:357-410` and `lctx-model/domain/serving/mappings.rs` (1,002, "Finite row views. Fields and nominal targets come from their canonical declaration").
- Cursors: `serving/cursor.rs` (Postcard continuations).
- Python: `lctx_mcp` reaches data only through the pyo3 `lctx_storage` (`open_service`, `RequestGrant`; `python/lctx_mcp/src/lctx_mcp/generation.py`). It has no direct DB driver; the search covered `psycopg|asyncpg|sqlalchemy|duckdb|lancedb` in `python/` (no hits apart from the lctx_storage path).
- Contract: §15.12, synthesis-and-serving §11.3–11.4, ADR-0114.
- *Interpretation:* the mechanism is a one-hop `= ANY($1)` lookup repeated across declared paths. That is generic and replaceable by graph traversal or batched joins. The path declarations, budgets, refusals and packet semantics are domain.

### (h) DataFusion integration and `lctx query`

| Component | src | Coupling and role |
|---|---|---|
| `cpg-core/src/generation_read.rs` | 920 | DataFusion `TableProvider` over the owned fork's `PostgresConnectionPool`/`SessionBinder`; tokio-postgres lease driver; closed predicate algebra for pushdown via `Unparser` with `PostgreSqlDialect` (`:646-760`); session never reconnects (P1.10, T13) |
| `cpg-core/src/model_runtime.rs` + `model_runtime/prepared.rs` | 368 + 217 | `SessionContext` config, stage registration, call-policy views, memory pool admission |
| `cpg-core/src/sql.rs` | 26 | read-only `SQLOptions` gate |
| `crates/lctx/src/query.rs` | 46 | `lctx query` |
| `third_party/datafusion-table-providers-df55.patch` | 1,398 (10 files) | owned fork `paul-heyse/datafusion-table-providers@a41da224` (`docs/pins.md:102`): Closing/Closed state, drain/release, cancellation guard, bounded chunks |

- The 19 compile-time query sites, all `SELECT * FROM "<rel>"` (the last one uses `ORDER BY id`):
  - `analysis_prepare.rs:73`, `synthesis_preparation.rs:18`, `final_coverage.rs:23`, `retrieval_preparation.rs:17`;
  - `catalog_core.rs:26,209`, `analysis_graphs.rs:73`, `semantic_models.rs:30`, `semantic_execution.rs:37`;
  - `normalize/mod.rs:22`, `consumed_rows.rs:126`, `catalog_evidence.rs:26`, `analytic_text.rs:18`;
  - `embedding_realization.rs:82`, `structural.rs:31`, `retrieval.rs:30`, `analytic_embedding.rs:29`;
  - `catalog_selection.rs:26,67`, `analysis_report.rs:78`.
- DESIGN §15.11 (Compute).
- *Interpretation:* about 1.5k lines plus a 1.4k-line fork exist to stream whole relations into Rust. The DataFusion query engine itself is essentially unused for computation, apart from `lctx query` inspection and the policy views.

### (i) Operational services

- `operations.rs` (116; `lctx_ops.attempts`/`events`, both 0 rows). `cache.rs` is listed under (f).
- `scripts/postgres_bootstrap.py` (190), `postgres_backup.py` (102), `postgres_recovery.py` (365).
- Generic.

### (j) Test and qualification harness

| Component | Lines |
|---|---|
| `lctx-postgres/src/testing.rs` (testcontainers PG18 `DisposableDatabase`, lifecycle `Harness`, `Small`/`Facts` fixtures) | 991 |
| Integration tests: `lctx-postgres/tests` | ≈10.1k |
| Integration tests: `cpg-core/tests` | ≈14.8k |
| Integration tests: `lctx/tests` | ≈9.9k |
| Inline tests in `lctx-postgres` src | 1,455 |
| `scripts/verify.py` (families `store`, `serving`, `qualify`; store family = 10 lctx-postgres test targets + `tests/scripts/test_postgres_serving.py`, `:55-84`) | 354 |
| `scripts/qualify_serving.py` | 852 |
| `scripts/postgres_test_support.py` | 104 |

The justfile has `verify-store` (`:61`), `verify-serving` (`:65`), `qualify` (`:77`), `store-check` (`:110`) and `postgres-test-setup`/`-ready` (`:251-266`). Pinned images are in `specs/postgres-image.txt` and `specs/postgres-vector-image.txt`.

### (k) Other

`lctx` CLI `store.rs` (88), `generation.rs` (194), `database.rs` (60) and `serving.rs` (36) are thin.

`cpg-core` stage glue: about 30 files use `session.register` / `reader.table` / `execute_stream` / `decode` to pull inputs. `normalize/mod.rs` has the highest density (30 matches). `compilation.rs` (869 src) and `facts.rs` (401) drive `GenerationAttempt` through `copy`, `seal` and `publish`. This is orchestration whose shape follows the stage table (domain), with substrate calls threaded through.

## 2. Live catalog facts (read-only, 2026-10-05)

Schema `lctx_gd3a3fa026a1237a1c4e175b9b1019eeb` (staging, catalog frontier, interrupted, 0 receipts).

**Relations**

- 1,086 tables: 784 canonical and 302 `__delta_*` vocabulary deltas.
- 126 views: 84 vocabulary-prefix views, 5 JOIN views (CallPolicy) and 37 projection/mapping/derivation views.
- 1,061 indexes, of which 917 are unique and 144 are `serving_*` lookup indexes.
- 0 sequences.
- Total size 2,308 MB. `reltuples` sum ≈ 11.10M.

**Table population**

| Category | Count |
|---|---|
| Canonical tables with rows | 187 |
| Canonical tables with 0 live tuples | 597 |
| Delta tables (all empty) | 302 |
| `reltuples` > 100k | 28 tables |
| `reltuples` 1k–100k | 120 tables |
| `reltuples` < 1k | 22 tables |

**Constraints**

| Type | Count | Notes |
|---|---|---|
| PRIMARY KEY | 784 | all composite `(generation_id, id)`; delta tables have none |
| UNIQUE | 133 | sum-type `(generation_id, id, tag)` |
| NOT NULL | 4,711 | |
| CHECK | 6,274 | see breakdown below |
| FOREIGN KEY | 12 | all two-column, all to the control schema (`introduced_epoch` → `publication_groups`); 0 intra-generation, 0 self-referencing |

CHECK breakdown:
- 2,809 Id-width checks on non-`id` columns. This approximates the number of nominal Id fields, so it is an upper bound on the reference FKs that would appear at `validated`.
- 784 generation-literal checks.
- 784 row-wire-bytes checks.
- 653 code-domain checks.
- 279 digest-width checks.
- 22 finite-float checks.
- 12 list-shape checks.
- The remainder are sum-arm and other checks.

**Self-referencing and cyclic FKs: not measurable here.** The reference FKs are absent because the generation is staging. DESIGN §15.11 states "Generation-qualified local references support cycles; tables/keys are created before foreign keys". Counting self-referencing or cyclic references requires a validated generation or reading the model's `field.target()` inventory. Not done.

**Column types**

| Type | Columns |
|---|---|
| bytea | 6,202 |
| smallint (codes) | 1,092 |
| bigint | 286 |
| text | 246 |
| boolean | 219 |
| double precision | 41 |
| text[] | 11 |
| integer | 4 |
| bigint[] | 1 |
| integer[] | 1 |

There are no jsonb, numeric, vector or bytea[] columns in the generation. List-of-Id fields do not exist, so every edge is a row.

**Largest relations**

| Relation | Rows | Size |
|---|---|---|
| `evidence` | 976k | 222 MB |
| `occurrences` | 959k | 215 MB |
| `entity_refs` | 1.04M | 210 MB |
| `syntax_placement_supports` | 959k | 199 MB |
| `syntax_placements` | 959k | 185 MB |
| `occurrence_ownership` | 959k | 161 MB |
| `syntax_supports` / `syntax_observations` | 317k | — |
| `type_supports` / `type_observations` | 214k | — |
| `call_target_*` / `call_resolution_*` | ≈141k | — |
| `artifact_chunks` | 9.9k | 29 MB |

Pattern: observation/support pairs, i.e. reified provenance edges.

**Other schemas**

- Control schema `lctx_model_store`: 17 tables (generations, receipts, publication_groups, epoch_receipts, …).
- `lctx_cache`: 4 tables, all 0 rows. `serving_vectors.value` is `lctx_ext.vector(1024)` with a btree PK only.
- `lctx_ops`: 2 tables, 0 rows.
- `lctx_model_store.selection`: no generation selected.

## 3. How much of `lctx-model` is storage-shaped?

`lctx-model` src is ≈124.2k lines plus ≈11.7k inline tests. The crate has **no sqlx or PostgreSQL dependency** (`crates/lctx-model/Cargo.toml`). Its dependencies are arrow, serde_arrow, petgraph, leiden-rs, biodivine-lib-bdd and postcard.

| Share | Lines | What |
|---|---|---|
| Arrow/physical lowering and in-memory store | ≈2.6k (≈2%) | `record.rs` 323, `model.rs` 941, `memory.rs` 703, `batching.rs` 193, `charged.rs` 216, `resources.rs` 215 |
| Macro-generated codecs | — | 930-line macro crate |
| Serving row-view declarations | 1.0k | `serving/mappings.rs` |
| Publication/stage/admission contracts | ≈4.6k (≈4%) | `stages.rs` 2,365, `validation.rs` 1,137, `admission` 1,133. Generation-lifecycle-shaped but database-agnostic in code. |
| Domain logic over typed rows | ≈116k (≈93%) | everything else |

The 93% is relationally *expressed*: 528 `Domain` records, observation/support reification, and Id-keyed in-memory indexes (84 `BTreeMap/HashMap<Id<…>>` sites in 15 files). Moving to another database would not change it unless the domain stopped being declared relations.

Only about 11 non-test lines in `lctx-model` mention SQL or PostgreSQL. The main one is `normalized/events.rs:56` (`CallPolicy::select_sql`).

## 4. Brittleness evidence (git, 2026-09-14 → 2026-10-05)

**Volume**

- 1,141 repository commits in the window.
- 537 touch `lctx-postgres` or `cpg-core` at all.
- 163 touch the narrower store/read surface: `lctx-postgres`, `generation_read.rs`, `model_runtime*`, `lctx_storage`, the fork patch, `postgres_*` and `qualify_serving` scripts.

**Churn**

- `lctx-postgres`: +57.7k / −24.4k.
- `cpg-core`: +92.8k / −68.7k.
- 152 files deleted.
- Deliberate rewrites (planned cutover, not defects):
  - Delta runtime and partition kernel removed: f6562d35, 093a31fb (2026-09-29).
  - PG12–PG15 import/hydration modules added 2026-09-28 (42889400), then superseded by canonical serving: 91751d1c (2026-10-02), 72ba0413.

**Fix-like commits on the narrow surface: 34 of 163.** Classified by the files they changed and their messages:

| Theme | Count | Example SHAs | Substrate or domain |
|---|---|---|---|
| Test harness, fixtures, qualification wiring | ≈12 | 5db685fe, 5671c899, d1fd1c00, ca05f18a, 27d09919, d099aeba, ceee005b, 1964c11f, f3a17009 | Mostly harness for typed premises and fixtures (domain-shaped fixtures); some PG-harness lifetimes |
| Lifecycle / receipts / validation-session acknowledgement | 6 | 425022b3 (checkpoint frame acknowledgements; `control.sql` + receipts + validation_session +178), 48e4fdd0, 2b42484d (ADR-0126 proof reuse, audit +190), cc89c918 | **Substrate-integration protocol** (receipts and frames), driven by domain authority rules |
| Store-lifecycle and provider-session review corrections | 2 large | 5beafd87 (attempt-only lifecycle, phased reset, typed failures, locks; store-lifecycle review F01–F07), 7c1594d2 (provider session F01–F06; `generation_read.rs` 120 lines) | **Substrate integration**: concurrency, locks, leases, fork-session lifecycle |
| Vocabulary epochs / prefix / ACL | 3 | d4f0d593 (vocabulary authority: ddl/stage_validation/verify/vocabulary +~300), 5262be03 (ADR-0108 boundaries vs prefix order), d099aeba (prefix ACL inspection) | Domain rule realised through PG deltas, views and grants (**mixed**) |
| Serving packet / hydration semantics | ≈8 | 29b39dfd, 3591b68c, ae9af4e6, 233db701, 1d0b9710, 11420bc0, 911f4279, 747bd10a | **Domain semantics** (proofs, scope, evidence citation) |
| Resource budget / CPU admission | 3 | 33cdec82, 8e031e1d, 3591b68c | Domain resource policy around substrate reads (mixed) |
| Physical mapping / connection | 1 | 6e7f0610 (`physical_columns`, IPv6 loopback) | Substrate |
| Recovery script / model-digest migration | 1 | 38efdd4b (`postgres_recovery.py` +152) | Substrate ops |
| COPY / codec / pgpq | 0 | — | No COPY or codec defects in the window |
| Constraint / FK | 0 explicit | (reference trials appear inside vocabulary fixes) | — |

Fork / `generation_read.rs` commits in the window: 10, of which 1 was a correction (7c1594d2). The fork patch's pins row lists lifecycle fixes made in the fork itself (`docs/pins.md:102`).

Concentration of edits:
- Lifecycle/validation core files (`validation_session`, `receipts`, `lifecycle`, `vocabulary`, `stage_validation`, `control.sql`, `ddl`, `lease`, `locks`, `install`, `verify`): 43 commits.
- Serving services: 38 commits.
- `testing.rs`: 25 commits.

**Reading of the evidence (interpretation)**

- Recurring substrate-integration defects concentrate in **concurrency and immutability choreography**: attempt locks, lease/drain, grants and revokes, checkpoint frames, vocabulary deltas and prefix views, provider-session lifecycle. They do not concentrate in load or codec.
- Most of this choreography exists to provide immutable, digest-bound, privilege-enforced snapshots inside a mutable server. Any database would need an equivalent (or a native snapshot or immutability feature) to drop it.
- Domain-semantic fixes, in serving hydration and proofs, are about as frequent and are database-independent.
- Commit messages frequently report `tests not_run` or `runtime pending`. Several defects are qualification-harness issues, not product defects.

## 5. Generic-vs-domain tally (approximate src lines)

| Bucket | Lines | Notes |
|---|---|---|
| Generic store mechanism, plausibly provided by a database | ≈9–10k + 1.4k fork patch + ≈2.3k snapshot code | lifecycle/locks/leases/roles/install/verify ≈4.8k; DDL ≈0.7k; COPY/codec ≈1.3k; DataFusion provider/runtime ≈1.5k; vectors/cache ≈0.85k; ops 0.1k |
| Generic part of serving hydration | ≈1–2k of the 11.2k | one-hop set-lookup chaining |
| Domain semantics that remain under any store | ≈125k+ | packet/section semantics, budgets, refusals, ranking; model validators, invariants, stage table, admission; the full `lctx-model` domain |

## 6. Material uncertainties and unresolved edges

1. **The live generation is an interrupted staging catalog generation.** FK counts, emptiness and sizes are not those of a published generation. Whether a published catalog generation currently exists anywhere: not checked beyond this database (registry has 1 row).
2. **The cyclic/self-reference count is unmeasured.** Obtaining it needs the model's reference inventory (`Relation::fields()[].target()`) or a validated generation.
3. **Theme classification uses commit messages plus touched files.** Diffs were not read line by line, apart from the stat listings. Counts are ±2 per theme.
4. **No performance evidence was gathered.** Two items could matter for a "slow" claim but were not measured:
   - row-at-a-time pgpq encoding (`mod.rs:655-680`);
   - sequential one-hop hydration round trips (252 call sites).
5. **Absence-claim coverage:**
   - "No JOIN, GROUP BY or recursive compute SQL": `crates/*/src/**/*.rs` only (grep `JOIN`, `GROUP BY`, `WITH RECURSIVE`). Python and scripts were not searched for compute SQL.
   - "lctx_mcp has no DB driver": `python/**/*.py`.
   - "DataFusion only `SELECT *`": `.query(` call sites in `cpg-core/src`. Tests and `lctx query` user SQL are excluded by design.
6. **`lctx-model` storage share is estimated by module and file classification** plus Arrow/SQL token counts. Macro-expanded code (serde_arrow conversions per record) is not counted in the model's lines.
