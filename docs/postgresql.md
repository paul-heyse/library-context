# PostgreSQL operations

**Implemented, 2026-09-27; qualification receipts:**
[PostgreSQL evidence](design_review/evidence/2026-09-27_postgresql/README.md).
[ADR-0065](adr/0065-postgresql-services-and-vector-receipts.md) owns the SQLx boundary;
[the plan](plans/postgresql-integration-plan_2026-09-27.md) records initial acceptance and
owns conditional capabilities. These instructions concern one operator's local application database.

## Local deployment and configuration

The installed `18/main` PostgreSQL 18.6 cluster on port 5432 hosts database `lctx`.
`lctx_migrator` owns the database/migrations; `lctx_app` is a separate limited login.
The bootstrap leaves host authentication unchanged and uses loopback SCRAM. Neither login has
superuser, createdb, createrole or replication privileges. `16/main` and unrelated databases
are outside this application deployment. No PostgreSQL extension is required.

For a new deployment, run `uv run python scripts/postgres_bootstrap.py` interactively once.
It requires postgres-administrator access through sudo. It refuses existing roles/database or
configuration rather than rotating credentials implicitly. On partial failure it retains the
protected generated credentials for operator diagnosis. Never rerun it as a repair/rotation tool.

Runtime configuration precedence is `--database-config PATH`, `LCTX_DATABASE_CONFIG`, then
`~/.config/library-context/postgres.json`. The JSON file must exclude group/other permissions
(`chmod 600`). It contains `application_url`, `migration_url`, `max_connections` (default 6),
`acquire_timeout_seconds` (5), `statement_timeout_seconds` (30), `lock_timeout_seconds` (5),
and `max_receipt_bytes` (268435456). Upper bounds are 32 connections, 60/300/60 seconds, and
1 GiB of retained vector payload. Map/text/returned-vector overhead is additional memory.
Pool idle lifetime is 60 seconds, maximum connection lifetime 30 minutes, and idle transactions
are limited to 30 seconds. Remote URLs require `sslmode=verify-full` and explicit trusted roots.

URLs/passwords are never CLI arguments, traced queries or committed files. Rust driver errors
expose only a class and SQLSTATE. `DATABASE_URL` belongs to SQLx metadata tooling; it is not
runtime application configuration. A config/credential/role error is distinct from a missing
migration. Do not solve it by weakening HBA authentication or granting superuser to the app.

```sh
cargo build --release -p lctx
target/release/lctx db migrate
target/release/lctx db check
target/release/lctx db status
```

Migration is explicit, embeds versioned SQL, serializes concurrent migrators with SQLx's
PostgreSQL advisory lock and checks migration checksums. Runtime commands never apply DDL.
The application role can SELECT/INSERT cache values and history, UPDATE rebuildable indexes,
and read migration status; it cannot UPDATE/DELETE winners or durable history, or create tables.

## Consumers and recovery

`lctx compile fastmcp --store build/store-pg --embedder fake|vllm` requires the configured cache.
There is no automatic fallback after failure. `--embedder none`, extraction, Delta query/diff,
bundle rebuild and file/native serving remain usable without PostgreSQL. Optional journaling is
disabled with a diagnostic when schema/connectivity is unavailable; that cannot unpublish Delta.

```sh
target/release/lctx runs list --limit 50 --offset 0
target/release/lctx runs show ATTEMPT_HEX
target/release/lctx snapshots list
target/release/lctx generations list
target/release/lctx db reconcile --store build/store-pg --generations build/generations
```

A missing terminal event means `unfinished`, not failed/interrupted. After confirming the process
stopped, `lctx runs mark-interrupted ATTEMPT_HEX` records the operator's observation. Reconciliation
reads canonical Delta publication and verified manifest identities. It adds recovered publication
observations; missing original start/library fields stay unknown. It does not reconstruct lost
operator events. Stale discovery locations become unavailable; historical events remain. Newer
concurrent discoveries survive reconciliation. Shared generation roots may contain other stores;
only matching canonical publications enter this store's index. Reconcile both old/new paths after
a move; query missing old paths by their same absolute spelling. Lists/show verify canonical
metadata rather than accepting PostgreSQL as publication authority.

Runtime tracing target `lctx::postgres` reports cache-read requested/hit counts, insert candidates/
inserted counts, vector bytes, duration and receipt payload/budget. Enable it through `LCTX_LOG=lctx::postgres=info` (the existing CLI subscriber).
It does not emit request text, vectors or connection strings. Failed journaling is reported on
stderr, preserving the original compile result. Use `db status` for effective role, PG version,
schema compatibility and pool state; use normal PostgreSQL statistics for backend/WAL/disk use.

For a legacy cache import, stop old writers and pin its Delta version. Supply a JSON array of
exact original document request texts reconstructed from pinned inputs, then run:

```sh
target/release/lctx db import-cache --store OLD_STORE --version VERSION \
  --requests REQUESTS_JSON --embedder vllm
```

Import validates the schema, canonical spec, original tokenizer admission, bytes and existing
winner equality. It reads requested keys in chunks of 32, reports inactive rows lacking request
text, and emits a source/version/spec/count/digest receipt. Retry is idempotent; conflicting
committed values fail explicitly. Keep the receipt with migration evidence. A live compile
re-admits misses itself. Old `embedding_cache` data is read only by this explicit importer.

## Backup, restore, retention and upgrades

Back up before relying on history, then daily and before migrations/upgrades. Retain at least
seven daily backups and a verified pre-upgrade backup in an operator-protected location; copy
these to independent storage under the operator's existing backup policy. No automatic deletion
or new scheduling daemon is installed. Cache/discovery are rebuildable; attempt/event history
is not. Canonical Delta stores/generations and protected credentials have their own backups.

```sh
uv run python scripts/postgres_backup.py backup BACKUP_DIRECTORY/lctx-YYYYMMDD.dump
uv run python scripts/postgres_backup.py restore-drill BACKUP_DIRECTORY/lctx-YYYYMMDD.dump
```

The backup uses PG18 `pg_dump -Fc` under an exported repeatable-read snapshot, with recovery
fingerprints for every application table from that same snapshot. UTC rendering makes timestamps
portable. The archive and JSON receipt are mode 600 and never overwritten. Roles/passwords are
not included. The drill restores only into a newly created disposable PG18 container, compares
every table fingerprint and removes that container. It never replaces the installed database.
Initial target: daily recovery point, local restore within 15 minutes; measured scope is in the
linked evidence. This is a single-database logical backup, not host recovery/PITR or HA.

For an actual restore, stop application writers, preserve the damaged database, provision a new
database and matching roles, restore with PG18 `pg_restore --exit-on-error`, point a protected
config at it, run `db check`, compare the backup receipt, then reconcile canonical stores.
Prefer a new database to an in-place destructive restore. PostgreSQL rows never replace missing
Delta snapshots. Losing only cache data causes future recomputation; published receipts retain
exact prior values. Grant retention deletions only to explicit maintenance under the migration
identity, never the runtime role; do not delete durable history without a backed-up policy.

Patch upgrade: back up, test the same patch/image, stop writers, apply the host package upgrade
under its normal operator procedure, then `db check` and a compile/replay control. Major upgrades
need a new version decision and disposable qualification. Do not restart the shared host cluster
just to rehearse an application outage; tests interrupt their own containers. The installed
service is managed by the existing `postgresql@18-main` systemd unit.

Rollback preserves new receipts/history, stops new writers, and uses the retained prior binary,
configuration and prior-format store together. Never open a new-format store with the old binary
or reverse-migrate receipt tables into legacy analysis. Baseline assets are retained under
`build/postgresql-baseline-5e62353/`; no automatic cleanup occurs.

## Development checks

`just postgres-test-setup` explicitly pulls the patch/digest in `specs/postgres-image.txt`.
`just test-postgres` runs real PG18 tests through Testcontainers; missing Docker/image is `blocked`.
`just sqlx-prepare` regenerates `.sqlx` against a freshly migrated disposable schema;
`just sqlx-check` detects metadata drift. Both use pinned sqlx-cli 0.9.0. Normal builds default to
`SQLX_OFFLINE=true` even with an ambient DSN; metadata tooling deliberately sets it false.
The full `just test-all` includes these checks. Keep this repository's existing Cargo target/cache;
if the shell inherits another project's `CARGO_TARGET_DIR`, explicitly select this repository's
`target`. Do not clean the cache. `just pilot STORE LOG` and `just pilot-live STORE LOG` allow fresh
stores/logs while preserving earlier evaluation artifacts.
