# PostgreSQL operations

**Implemented and focused-Tested, 2026-09-29** (semantic-model cutover phase 1). PostgreSQL 18 is
the single relational store ([ADR-0086](adr/0086-immutable-postgresql-generations.md)):
- retained services: the embedding cache and the operational attempt history;
- immutable generation schemas.

[DESIGN §15.11](design/sections/semantic-model.md#section-15-11) owns the store's semantics. The
[cutover plan](plans/semantic-model-cutover-plan_2026-09-29.md) owns sequencing, receipts (§4.2) and
finding dispositions (§8). These instructions concern one operator's local database.

Serving (`lctx_serving`, projections and the MCP server) is dormant until cutover phase 5. Git holds
its former operations.

## Local deployment and configuration

The installed `18/main` PostgreSQL 18.6 cluster on port 5432 hosts database `lctx`. Its roles:

| Role | Owns or may do |
|---|---|
| `lctx_migrator` | Owns the database. It is the **service owner**: service baseline, control schema, generation schemas |
| `lctx_app` | Retained services: reads and inserts cache values and attempt history, reads migration status |
| `lctx_importer` | Writes staging generations. It holds TEMP and receives writer grants only while a generation is staging |
| `lctx_serving` | Reads published generations and every control table. Its transactions default to read-only |

None of them holds superuser, createdb, createrole, replication or bypassrls, and none is a member of
another. `lctx store check` verifies all of this.

For a new deployment:
1. Install `postgresql-18-pgvector=0.8.6-1.pgdg24.04+2`.
2. Run `uv run python scripts/postgres_bootstrap.py` once, interactively. It needs sudo access to
   `postgres`. It creates the database and the four roles and writes their protected configurations.
   It refuses existing roles, databases or configurations; it is not a rotation or repair tool.
3. Run `lctx store install`, then `lctx store check`.

Configuration discovery:
- `--database PATH` names the protected application configuration, `postgres.json`;
- otherwise `LCTX_DATABASE_CONFIG`, then `~/.config/library-context/postgres.json`.

Every file must be a regular mode-0600 file. The files beside it select the roles:

| File | Used for |
|---|---|
| `postgres.json` | The retained services (`lctx runs`); pool, timeout and receipt limits |
| `postgres-admin.json` | The service owner (`lctx store`, owner `lctx generation` commands). Pool of 4, 30-minute statement timeout for validation |
| `postgres-serving.json` | The reader (`lctx generation list/show`, `lctx query`). Six connections by default, two reserved for provider sessions |
| `postgres-importer.json` | The writer, used by attempts (P2 `lctx compile`) |

Credentials:
- URLs and passwords are never CLI arguments, traced queries or committed files.
- Remote URLs require `sslmode=verify-full` with explicit trusted roots.
- Driver errors expose only a class and SQLSTATE.
- `DATABASE_URL` belongs to SQLx metadata tooling, not runtime configuration.

## The generation store

```sh
cargo build --release -p lctx
target/release/lctx store install     # apply the service baseline; install or confirm the store
target/release/lctx store check       # compare the live catalog with this binary's lowering; exit 2 on findings
target/release/lctx store reset       # dry run: what a reset drops; exit 2
target/release/lctx store reset --confirm lctx
```

**Install.** `store install` applies the single service-baseline migration, then installs the
control schema `lctx_model_store` for this binary's model. A migration history that predates the
baseline is refused, never upgraded (see [The transition](#the-transition)). Runtime commands never
apply DDL.

**Check.** `store check` compares every generation schema, and the control schema, with a
rolled-back shadow install of this binary's lowering. It also checks:
- the roles and database privileges;
- the service migration history;
- the owner's schemas and objects;
- orphans in both directions.

**Reset.** `store reset` removes one generation per transaction and replaces the control schema
last. It is resumable: rerun it after an interruption.

**Busy refusals.** Install, check and reset try the installation lock and refuse `Busy` (exit 2)
rather than waiting behind work in flight.

Every generation belongs to the attempt that registers it. P2's `lctx compile --through facts`
drives attempts; nothing else advances a generation. Operators see and steer generations with:

```sh
target/release/lctx generation list [--state published] [--frontier facts]
target/release/lctx generation show GENERATION_HEX
target/release/lctx generation select GENERATION_HEX      # facts generations only; compile never selects
target/release/lctx generation clear-selection
target/release/lctx generation retire GENERATION_HEX      # unselected, unleased, published
target/release/lctx generation abort GENERATION_HEX       # failed or interrupted
target/release/lctx query --generation GENERATION_HEX "SELECT count(*) FROM occurrences"
```

`show` reports:
- state, frontier, profile and selection;
- digests and relation receipts;
- a facts generation's per-family availability;
- a failed generation's from-state, class and safe detail (SQLSTATE, constraint, table);
- the reader count;
- whether its attempt is live or interrupted.

`query` is read-only DataFusion SQL over one leased generation. DDL and DML are refused, and so is
a relation outside the generation's frontier.

Exit status: 0 ok, 1 error, 2 refused, 3 unavailable.

## Retained services

```sh
target/release/lctx runs list --limit 50 --offset 0
target/release/lctx runs show ATTEMPT_HEX
target/release/lctx runs mark-interrupted ATTEMPT_HEX     # after confirming the process stopped
```

A missing terminal event means `unfinished`, not failed or interrupted. Tracing target
`lctx::postgres` (`LCTX_LOG=lctx::postgres=info`) reports cache reads and admissions without request
text, vectors or connection strings.

## Backup, restore and upgrades

This is the design-phase policy ([ADR-0078](adr/0078-current-design-cutover.md)). Generations are
rebuilt from pinned inputs, never restored across formats.

- `scripts/postgres_backup.py backup|restore-drill` fingerprints the retained service tables.
- A model change is a `store reset` followed by a rebuild.
- An old-format database moves through [the transition](#the-transition).

## The transition

A database carrying the pre-baseline history (migrations 0001–0013) moves offline, with
`scripts/postgres_transition.py`. Server administration runs as `postgres` through sudo on `--port`,
or through a protected `--admin-config` JSON file `{"url": ...}`.

```sh
uv run python scripts/postgres_transition.py plan         # read-only: history, retained rows, legacy schemas
uv run python scripts/postgres_transition.py prepare      # create lctx_next, `lctx store install`, copy, verify fingerprints
# stop every reader and writer
uv run python scripts/postgres_transition.py switch --confirm-switch lctx
uv run python scripts/postgres_transition.py drop-retired --confirm-drop lctx_retired_YYYYMMDDHHMMSS
```

- The retained rows are the cache specs and values, and the attempts and events.
- The old database is never written. `switch` renames it to an archive only after confirming that no
  connection remains, then checks the new database (`store check`, `runs list`, equal fingerprints).
- Dropping the archive is a separate, explicit step.
- Refusals exit 2 and change nothing.

## Development checks

- `just postgres-test-setup` pulls the pinned images.
- `just test-postgres` runs the real-PG18 suites through Testcontainers: the store, provider sessions,
  the CLI and the transition. Missing Docker or a missing image is `blocked`.
- The `testing` feature of `lctx-postgres` provides `DisposableDatabase`, which is provisioned like
  production with production session limits, and the attempt-semantics test harness.
- `just sqlx-check` and `just sqlx-prepare` cover the dormant serving queries frozen in `.sqlx`. They
  leave `test-all` until serving returns (plan T12).
- Keep this repository's Cargo target and build cache; never clean them.
