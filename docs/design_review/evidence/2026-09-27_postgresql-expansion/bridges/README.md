# PostgreSQL bridge compatibility probes

Date: 2026-09-27. These are isolated design probes, not production integration or release acceptance.
The parent [review](../../../reviews/design_review_postgresql-expanded-architecture_2026-09-27.md) owns
recommendations. Root manifests, lockfile, PostgreSQL configuration and plans were not changed.
This independent Cargo workspace resolves candidate boundaries together, while optional features
keep transport, analytical providers and server-facing adapters independently selectable.

## Receipts

- **Tested — passed:** `CARGO_TARGET_DIR=/home/paul/.cache/lctx-pg-expansion-target cargo metadata --all-features --format-version 1 --manifest-path docs/design_review/evidence/2026-09-27_postgresql-expansion/bridges/Cargo.toml`.
  All listed candidates, SQLx 0.9 and the repository's exact delta-rs commit resolve together with
  one DataFusion 55.1.0, Arrow 59.3.0, object_store 0.13.2, ADBC 0.24.0 and federation 0.5.7.
  [Resolved family](raw/resolved-family.json), independent [lockfile](Cargo.lock).
  This is dependency resolution, not proof that all optional packages compile or work.
- **Tested — passed:** the direct `pgpq`/SQLx probe, built with the default feature set and run
  against a disposable digest-pinned PostgreSQL 18.6 container. Exact 16-byte IDs, Float32 negative
  zero, empty/NULL/literal-`null` text, nonempty/empty/NULL arrays with nullable elements, explicit
  COPY abort and subsequent connection reuse all pass. [Output](raw/copy-run.log).
  The probe uses public `ArrowToPostgresBinaryEncoder` header/batch/footer and SQLx
  `PgConnection::copy_in_raw`/`send`/`finish`; no additional Rust-Postgres driver is needed.
- **Tested — passed:** `CARGO_TARGET_DIR=/home/paul/.cache/lctx-pg-expansion-target cargo check --release --locked --features datafusion,datafusion-federation,datafusion-table-providers-postgres,datafusion-table-providers-adbc,deltalake --manifest-path docs/design_review/evidence/2026-09-27_postgresql-expansion/bridges/Cargo.toml`.
  Both migrated providers, federation, SQLx, pgpq and the exact delta-rs revision compile together.
  [Output](raw/provider-check.log). Native ADBC driver loading/execution was **not_run**.
- **Tested — passed:** `python3 docs/design_review/evidence/2026-09-27_postgresql-expansion/bridges/run.py --providers`.
  Real PostgreSQL provider + federation on DF55.1/Arrow59.3 executes a null filter with limit,
  empty result and same-source join. BYTEA comes back as Binary; the provider's `source_type=bytea`
  annotation is present, while original `lctx.probe=identity` metadata is not recovered.
  This confirms a normalization boundary, not universal metadata loss. [Output](raw/provider-run.log).

Reproduce the direct transport probe with `uv run python docs/design_review/evidence/2026-09-27_postgresql-expansion/bridges/run.py`.
Use `--providers` to additionally exercise the DataFusion PostgreSQL provider and federation.
The script uses a dedicated disposable container and cache target, never the operator database.

## Source audit

**Interface-checked**, 2026-09-27:

| Candidate | Exact source and result | Capability direction |
|---|---|---|
| datafusion-federation 0.5.7 | [Published manifest](https://docs.rs/crate/datafusion-federation/0.5.7/source/Cargo.toml): DF `55`, Arrow JSON `59.2`; compatible ranges | Planner/provider federation; not a PostgreSQL client |
| datafusion-postgres family | [eda0da0 workspace](https://github.com/datafusion-contrib/datafusion-postgres/blob/eda0da032ed8d6003b5041fce67c1e5b2f101876/Cargo.toml): DF55/Arrow59 | PostgreSQL-wire **server over DataFusion**; `arrow-pg` codec, emulated catalog and PG functions serve that direction |
| datafusion-table-providers PostgreSQL and ADBC | [3309558 workspace](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/Cargo.toml): DF55/Arrow59, federation ^0.5.6 resolves0.5.7 | Read/write providers importing remote database data into DataFusion; PostgreSQL provider owns bb8/tokio-postgres/native-TLS; ADBC owns r2d2/driver-manager/FFI |
| ADBC Rust 0.24 | [core](https://docs.rs/crate/adbc_core/0.24.0/source/Cargo.toml), [manager](https://docs.rs/crate/adbc_driver_manager/0.24.0/source/Cargo.toml), [FFI](https://docs.rs/crate/adbc_ffi/0.24.0/source/Cargo.toml): Arrow >=58,<60 | Arrow-native driver API, dynamic client-driver loading and C interoperability |
| ADBC 0.25 development | [c942d48 Rust workspace](https://github.com/apache/arrow-adbc/blob/c942d481c6e083040c68676e3dd454dad89503e9/rust/Cargo.toml): >=58,<61 | Compatible range is not a reason to adopt development version over sufficient0.24 |
| ADBC PostgreSQL native driver | [pinned documentation](https://github.com/apache/arrow-adbc/blob/c942d481c6e083040c68676e3dd454dad89503e9/docs/source/driver/postgresql.rst) | libpq client, COPY reads/bulk ingestion/transactions; C ABI avoids arrow-rs version coupling but not semantic conversion or native artifact pinning |
| r2d2_adbc 0.3.0 | [manifest](https://docs.rs/crate/r2d2_adbc/0.3.0/source/Cargo.toml) uses `adbc_core >=0.21`; resolved0.24 in this lock | Synchronous pool; is_valid only creates a statement and has_broken alwaysfalse, so no crash/reuse guarantee follows from pool availability |
| adbc-driver-datafusion 0.27.0 source | [3d24e0f manifest](https://github.com/adbc-drivers/datafusion/blob/3d24e0f3ad8bf914b9d2a48d0151fc313b8ae28c/Cargo.toml): DF55.0/Arrow59.2/ADBC0.24 | Exposes embedded DataFusion **through ADBC**; not the PostgreSQL client driver |
| pgpq 0.12.0 | [manifest](https://docs.rs/crate/pgpq/0.12.0/source/Cargo.toml), [encoder](https://docs.rs/crate/pgpq/0.12.0/source/src/lib.rs) | Arrow→PostgreSQL binary COPY encoder, transport-independent; tested directly with SQLx |
| tokio-postgres 0.7.18 | [published API](https://docs.rs/tokio-postgres/0.7.18/tokio_postgres/) | Async control/transaction/COPY client, no Arrow/DF coupling; already an internal dependency of PostgreSQL provider, not needed as a second application control owner |
| connector_arrow 0.12.2 | [manifest](https://docs.rs/crate/connector_arrow/0.12.2/source/Cargo.toml): Arrow58 | Excluded from this shared-type family unless separately migrated |

Upstream table-providers `main` at `950a4792b72f3248c1986b1e1ddf6de617909d0c`
(2026-09-26) remains DF54/Arrow58. An owned, immutable fork derived from the supplied migration
is a credible adoption route, subject to focused conversion, optimizer and lifecycle qualification.
Following a moving `master`/`main` branch is not equivalent to pinning the supplied commit.

## Semantic boundaries found in exact source

These are **Interface-checked**, not broad runtime conformance claims.

- The PostgreSQL provider [catalog conversion](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/postgres/src/arrow_sql_gen/schema.rs#L66)
  maps BYTEA→Binary, UUID→Utf8 and PG arrays→List. It does not infer our FixedSizeBinary ID
  width, codebook integer widths, original field metadata or vector dimension contracts. The
  assembled runtime query retains the provider's `source_type` metadata; it does not recover
  metadata from the original input Arrow schema. Its
  [row decoder](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/postgres/src/arrow_sql_gen/mod.rs#L315)
  rebuilds fields and propagates nullability, but does not generally copy domain metadata or
  coerce every projected type. This needs a narrow schema-normalization adapter with checked
  width/null/shape rules, not a claim that Arrow compatibility makes interchange lossless.
- Provider pool configuration accepts explicit host/port/user/pass/db fields or its own libpq
  keyword parser; passing the existing SQLx URL directly to `connection_string` failed the probe.
  Adapt the existing protected configuration at the owning boundary, keeping redaction, TLS and
  total connection budgets explicit. This is a different pool/driver, not the SQLx pool reused.
- Provider reads use [query_raw plus 4,000-row conversion](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/postgres/src/conn.rs#L443),
  with COPY listed as a TODO. ADBC's COPY route is an alternative to benchmark if conversion
  dominates; the native PostgreSQL provider itself is not an Arrow-native PostgreSQL wire protocol.
- [Each SqlExec gets a pool connection](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/common/src/sql/sql_provider_datafusion/mod.rs#L681).
  [Federation groups a compute context](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/common/src/sql/sql_provider_datafusion/federation.rs#L66)
  but still gets a connection for each execution. Independent scans do not share one MVCC snapshot
  automatically. Immutable generation-qualified relations are a good fit; mutable operational
  observations must be explicitly diagnostic or materialized under a single declared transaction.
- [Filter pushdown](https://github.com/CaptainEureka/datafusion-table-providers/blob/33095588fcdd17301a5d1c340dcd66cd60e41ec8/crates/common/src/sql/sql_provider_datafusion/mod.rs#L259)
  labels successfully unparsed non-subquery expressions Exact. SQL expressibility alone does not
  prove PostgreSQL/DataFusion equivalence for collations, nulls, numeric/float comparisons,
  timestamps or casts. Admit a qualified expression subset and compare pushdown-on/off and
  filter/order/limit cases before semantic consumers depend on it.
- ADBC's [documented mappings](https://github.com/apache/arrow-adbc/blob/c942d481c6e083040c68676e3dd454dad89503e9/docs/source/driver/postgresql.rst#L245)
  include NUMERIC→Utf8, PG arrays→List and unknown types→opaque binary. Native-driver COPY,
  transaction and Arrow C ABI support do not restore lost repository-specific schema contracts.
- pgpq supports FixedSizeBinary, fixed/list arrays and composites, but does not implement a
  pgvector-specific encoder. For a vector(1024) projection, use a validated real[] staging
  column followed by an explicit vector cast, or a qualified native pgvector codec. It is a bulk
  transfer mechanism; SQLx remains owner of the transaction, staged load, validation and publication.

No performance comparison, whole-store replacement, full tool-answer parity or complete
pushdown/read-view/ADBC-native-driver conformance is claimed by these probes.
