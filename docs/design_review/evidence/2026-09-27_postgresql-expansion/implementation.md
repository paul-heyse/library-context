# PG8–PG11 implementation evidence

**2026-09-27.** This page records the bounded foundation slice under
[ADR-0068](../../../adr/0078-current-design-cutover.md) and the
[PostgreSQL plan](../../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream).
The forward plan §6.1 owns finding disposition. Production import/promotion, PostgreSQL MCP
query cutover, ANN and admitted federation remain PG12–PG17.

## Implemented contracts

- `cpg-schema` owns spec format 2, projection manifest format 1, all 51 FORMAT 11 relations,
  their schemas, logical receipts, domain keys and complete analytic support validation.
  Canonical compiler output is 107. Catalog 7 and native kernel 1 are unchanged.
- `lctx-postgres` owns SQLx effects, migrations, checked Arrow/COPY codecs, role-specific pools
  and unfinished-lease disposal. The semantic wheel remains pure; the new `lctx_storage` wheel
  provides explicit asynchronous open/check/close on one bounded Tokio runtime.
- PostgreSQL 18.6 has pgvector 0.8.6, four checksummed migrations and separate protected
  application, migrator, importer and read-only serving credentials. Migrations 001/002 were
  moved without changing their bytes. Migrations 003/004 declare metadata and typed relations;
  production importer/promotion methods are deliberately PG12 work.
- The provider fork is owned at `paul-heyse/datafusion-table-providers`, immutable revision
  `b4dbe895b40ad49eb0593d8887616eba2c1f9bbf`, based on the supplied DF55 migration. Its bounded,
  typed pool constructor is isolated behind `cpg_core::postgres_read`; production scans wait
  for PG15 qualification. Patch provenance is in `third_party/datafusion-table-providers-df55.patch`.

## Focused functional receipts

All commands below use the repository target `/home/paul/library-context/target`. The inherited
target belonged to another repository and was explicitly overridden. Logs and generated data
are under gitignored `build/`; backups/configuration contain credentials and are never published.

| Command / control | Outcome and boundary |
|---|---|
| `cargo test --release -p lctx-embed --test client`; focused Python embedder/launcher tests | **passed:** canonical spec/request known answers and response rejection controls; explicit dimensions 1024 |
| Controlled `scripts/embed_serve.py --port 8123`; Rust `live_conformance_vectors`; Python conformance | **passed:** actual pinned Qwen/vLLM requests at 1024; worst cross-client cosine 0.9999176 against registered ≥0.9995. Only this task's service was stopped |
| `lctx compile fastmcp --store build/postgresql-1024-live-cold --generations build/postgresql-1024-live-generations --embedder vllm --embed-url http://127.0.0.1:8123` | **passed:** fresh 1024 canonical snapshot `7b71debb80363846d3fb080d4ffd7e7a`, generation `0a5dcc1fa2534879`; total 215.4 s. Discovery was reconciled after correcting shared-buffer accounting |
| Same compile into `build/postgresql-1024-live-warm`, after stopping the endpoint | **passed:** snapshot `a27fde11262cc1ebf445a60d54b7a4b4`, generation `2a49db647d7c73f6`; total 149.5 s. Sorted Delta queries of spec/input/value/vector receipts are byte-equal (35,065,938 bytes, log `pg9-cold-warm-receipts.log`). Compiler source identity changed during the repair; whole canonical snapshot equality is not claimed |
| `LCTX_DATABASE_CONFIG=/nonexistent/pg9-offline.json lctx bundle --store build/postgresql-1024-live-cold --snapshot 7b71debb80363846d3fb080d4ffd7e7a --out build/postgresql-1024-live-replay` | **passed:** all 52 generation files byte-equal with PostgreSQL unavailable and the embedding endpoint stopped |
| Python `generation.load` of both live generations | **passed:** fresh native wheel agrees with compiler projection identity; 0.685/0.725 s. Sequential process peak 396,752 KiB includes retained full file hydration |
| `cargo test --release -p cpg-schema --test serving_projection` | **passed:** independent receipt framing, multiplicity/order controls, logical IPC memory accounting, missing/mixed manifest rejection and feature-independent definition digest |
| `uv run pytest python/lctx_mcp/tests/test_projection.py -q` | **passed:** independent Python receipts for every fixture relation; malformed domain keys, missing support, unknown codes and incompatible definition rejected after recomputing legitimate outer receipts |
| `cargo test --release -p lctx-postgres --test serving` | **passed:** real PG18 role/RLS invisibility, generation FKs, concurrent writer/freeze barrier, ready immutability, binary COPY/pgvector/Arrow round trips, empty schemas and malformed types; TLS/config/budget controls |
| Focused `cpg-core` provider pool test | **passed:** actual PostgreSQL provider and SQLx connections share a three-connection budget with one provider reservation |
| `LCTX_POSTGRES_TEST=1 uv run pytest tests/scripts/test_postgres_serving.py -q` | **passed:** concurrent awaitables, actual blocked-query cancellation, Python heartbeat, server-side drain before reuse, close/idempotence and sanitized errors |
| Administrator extension/role provisioning (now owned by `scripts/postgres_bootstrap.py`); `lctx db migrate/check/status` | **passed:** operator performed administrator provisioning; subsequent explicit migrations/checks report server 180006, `lctx_app`, current schema. Installed serving credentials also open/check/close successfully |
| PG7 baseline and expanded `postgres_backup.py restore-drill` drills | **passed:** separate disposable restores compare every recorded table fingerprint; preserved two-migration PG7 baseline and four-migration expanded schema remain distinct |

The real cold pilot exposed two cross-boundary defects, both corrected with focused regressions:
Arrow IPC columns can share allocation buffers, so allocation totals cannot bound logical batch
size; and serde_json feature unification can change object order, so identity encoding now uses
explicit ordered serialization fields. The standalone native wheel and root compiler agree on
definition digest `01f5b50e67bf65f16e9a9df89c2416c73c61c0d6663b4b903a089375978d95da`.

## Registered resource control

`cargo run --release -p lctx-postgres --example projection_probe -- <generation>/operation_vectors.arrow` exercised
real PostgreSQL COPY staging with the live operation-vector relation. **Measured:** 2,454 rows /
10,228,186 logical bytes took 0.174 s and peaked at 58,248 KiB; ten repetitions (24,540 rows /
102,281,824 bytes) took 1.303 s and peaked at 376,608 KiB, below the registered 512 MiB ceiling.
Round-trip logical receipts matched. This is a transport workload with repeated rows, not a
tenfold semantic corpus, production import transaction, ANN latency or WAL measurement.

## End-of-slice verification

**Passed, 2026-09-27; functional commits `1800348` and `fda9297`.** Formatting, broader checks
and documentation checks began only after the functional slice was implemented, as requested.
The [fresh change review](../../reviews/design_review_postgresql-foundations_2026-09-27.md)
concludes **Accept scoped** after correcting manifest/content/envelope identity binding and
empty dimension-zero codec preservation. Its F01/F02 disposition lives in forward-plan §6.1.

| Command / retained receipt | Outcome |
|---|---|
| `just test-all`, then a six-test `cargo nextest run --release --workspace -E ...` repair run | **passed across the complete no-fail-fast run and targeted repairs:** 437 ordinary Rust tests. Initial run retained 431 passes; six failures were stale consumers/source inventory/expected identities from the explicit migration. All six reran successfully; unaffected passing evidence was retained |
| `just fmt-check lint` | **passed:** final workspace Rustfmt/Clippy `-D warnings`, Ruff formatting/lint |
| `just py-check rules-scan rules-test lint-agents fixtures-check deps gold test-postgres sqlx-check` | **passed:** fresh Python fixture, 164 Python tests (one explicit PG test skipped here and run separately), Pyrefly, seven rules tests, agent checks, 83 fixture parses, one pinned family/cargo-deny/fork/shear/gold, 11 real PostgreSQL Rust tests, one real async Python test, and fresh-schema SQLx metadata |
| `uv run --no-sync python scripts/adr.py lint` | **passed:** 36 current records after predecessor retirement |
| `just pilot build/postgresql-pg11-pilot-store build/pg11-pilot.log` | **passed:** fresh standard fake-spec snapshot `f783ed5ba6e51720892c37a13bc77584`, generation `b45c37a56afcee34`, 143.0 s; MCP smoke over 20 briefs |
| `DATABASE_URL=postgres://invalid:invalid@127.0.0.1:1/unreachable SQLX_OFFLINE=true cargo check --release --workspace --all-targets --locked` | **passed:** ordinary graph compiles without a reachable database |
| Deliberately change one `.sqlx` nullable entry, run `scripts/postgres_check.py`, restore exact original bytes | **passed:** expected nonzero drift rejection identifies query `09cfdbfc2f56a22b445fdb7d1f92cb5e8c852499a38aba17d06753736b01e57d`; working metadata remains unchanged |
| `target/release/lctx db check`; `db status` | **passed:** rebuilt local CLI sees server 180006, `lctx_app`, current schema |
| `just docs-check`; scoped same publisher | **failed:** unchanged supplied external review has no H1. **passed:** publisher excluding only that input, 150 canonical pages and zero link errors |

Logs are `build/pg11-test-all.log`, `pg11-failed-rerun.log`, `pg11-gate-continuation.log`,
`pg11-final-lint.log`, `pg11-pilot-command.log`, `pg10-offline-build.log`,
`pg10-stale-metadata.log`, and `pg11-docs-scoped.log`. This is the PG8–PG11 code/deployment
checkpoint, not PG17 assembled import/query/ANN/federation acceptance or Stage 3 completion.
The kNN invocation snapshot was read before `cargo insta accept`: only the format-2 spec hash
changed; parameters, 28 rows and result counts were unchanged. Compiler-107 output identity
and the all-techniques guard were reviewed and advanced without changing thresholds or models.
