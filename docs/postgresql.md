# PostgreSQL operations

**Implemented and bounded Tested, 2026-09-28; live embedding waived for PR4:**
[current evidence](design_review/evidence/2026-09-28_pr4/README.md).
[ADR-0078](adr/0078-current-design-cutover.md) owns the SQLx boundary;
[the plan](plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream) owns integration and
owns conditional capabilities. These instructions concern one operator's local application database.

## Local deployment and configuration

The installed `18/main` PostgreSQL 18.6 cluster on port 5432 hosts database `lctx`.
`lctx_migrator` owns the database/migrations; `lctx_app` is a separate limited login.
The bootstrap leaves host authentication unchanged and uses loopback SCRAM. Neither login has
superuser, createdb, createrole or replication privileges. Unrelated databases are outside this application deployment. pgvector 0.8.6 is installed explicitly in the protected `lctx_ext` schema.

For a new deployment, install the pinned extension package with
`sudo apt-get install postgresql-18-pgvector=0.8.6-1.pgdg24.04+2`, then run
`uv run python scripts/postgres_bootstrap.py` interactively once.
Bootstrap creates the application, migration, importer and serving roles, installs the extension
in `lctx_ext` and writes the four separate credential files listed below. It does not run migrations.
It requires postgres-administrator access through sudo. It refuses existing roles/database or
configuration rather than rotating credentials implicitly. On partial failure it retains the
protected generated credentials for operator diagnosis. Never rerun it as a repair/rotation tool.

Runtime configuration precedence is `--database-config PATH`, `LCTX_DATABASE_CONFIG`, then
`~/.config/library-context/postgres.json`. The JSON file must exclude group/other permissions
(`chmod 600`). It contains `application_url`, `max_connections` (default 6),
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

## Development environment

Use the root editable uv environment for Python and the existing cached Cargo target for Rust.
Rebuild native bindings in that environment when their Rust sources change. PostgreSQL requires
`lctx_storage` and the pure `lctx_semantics` bindings, not standalone wheel artifacts; packaging
and clean-install checks are deferred until a distribution workflow needs them.

## Consumers and recovery

`lctx compile fastmcp --store build/store-pg --embedder fake|vllm` requires the configured cache.
There is no automatic fallback after failure. `--embedder none`, extraction, Delta query/diff,
bundle rebuild and pure native semantic evaluation remain usable without PostgreSQL. Online MCP
serving requires a ready PostgreSQL generation. Optional journaling is
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

## Backup, restore, retention and upgrades

**Accepted design-phase policy, 2026-09-28; [ADR-0078](adr/0078-current-design-cutover.md).**
Validate a fresh current-schema build before changing operator state. Then stop project readers
and writers, replace the application database/store and serving artifacts from pinned inputs,
serve the validated current generation, and delete obsolete generations, runtime copies and
rollback dumps. Old-format or mixed-version restore is not supported. Temporary baselines exist
only while needed by current qualification.

A current-format backup/restore or cold-reconstruction test can verify the current artifact
closure and selected generation. It creates no obligation to retain obsolete runtime epochs.
Project acquisition inputs, current provenance and unexecuted evaluation inputs remain inputs;
unrelated databases and workspaces are outside the reset.

## Development checks

`just postgres-test-setup` explicitly pulls the patch/digest in `specs/postgres-image.txt`.
`just test-postgres` runs real PG18 tests through Testcontainers; missing Docker/image is `blocked`.
`just sqlx-prepare` regenerates `.sqlx` against a freshly migrated disposable schema;
`just sqlx-check` detects metadata drift. Both use pinned sqlx-cli 0.9.0. Normal builds default to
`SQLX_OFFLINE=true` even with an ambient DSN; metadata tooling deliberately sets it false.
The full `just test-all` includes these checks. Keep this repository's existing Cargo target/cache;
if the shell inherits another project's `CARGO_TARGET_DIR`, explicitly select this repository's
`target`. Do not clean the cache. `just pilot STORE LOG` and `just pilot-live STORE LOG` allow fresh
stores/logs without overwriting the current generation.

## Role separation

Protected files in `~/.config/library-context/` now separate capabilities:

| File | Capability |
|---|---|
| `postgres.json` | Existing cache/operations application connection and limits |
| `postgres-admin.json` | Migration connection only; explicit commands and protected backup tooling |
| `postgres-importer.json` | At most two import connections; TEMP staging allowed, application schema DDL denied |
| `postgres-serving.json` | Read-only role; six total connections by default, zero provider connections until selected |

All are regular mode-0600 files. The obsolete upgrade script and its pending credential copy
are removed after validated cutover. Never copy credentials into command lines, logs or reports. Both clients use `pg_catalog,lctx_ext` as their trusted search path. Remote connections
require hostname-verifying TLS; a provider budget is reserved from the total, not added to it.
Runtime statements/locks/acquisition and cancelled leases are bounded. `lctx_storage` opens only
when explicitly awaited and closes at lifespan exit; importing the module makes no connection.

The SQLx migration history now lives in `crates/lctx-postgres/migrations`; the original two
migrations are byte-preserved. Projection schema installation does not publish a ready generation.
PG12 owns load/freeze/validation/promotion and child vector partition creation. Import uses TEMP
COPY followed by generation-qualified inserts: RLS protects final tables. No ready-generation
cleanup is available. Native/lexical artifacts remain digest-checked files with manifest references.

## Publication and queries

PR4's current format is bundle16/projection6/wire3 with migration012. Only the current format is
accepted. The catalog profile and behavioral enrichment use the same typed member catalog and
addressable retrieval artifacts. Exact PostgreSQL cosine is the sole current vector route.

After the coordinated explicit `lctx db migrate`/`lctx db check`, publication uses:

```sh
target/release/lctx serving import --store STORE --snapshot SNAPSHOT_ID \
  --bundles build/generations --artifacts build/serving-artifacts
# Or import an existing verified portable export:
target/release/lctx serving import-bundle --bundle BUNDLE_DIRECTORY \
  --artifacts build/serving-artifacts
target/release/lctx serving status --generation FULL_GENERATION_DIGEST
target/release/lctx serving reconcile --generation FULL_GENERATION_DIGEST
target/release/lctx serving select --library fastmcp --generation FULL_GENERATION_DIGEST
uv run lctx-mcp --library fastmcp --embedder none
```

`serving --importer-config FILE COMMAND` selects an explicit protected importer file. Defaults
use the sibling `postgres-importer.json` of `--database-config`/`LCTX_DATABASE_CONFIG`.
`lctx-mcp --config FILE --library NAME --generation FULL_DIGEST --profile PROFILE_DIGEST`
pins an explicit ready generation/profile; omitted generation uses the selected pointer once at
startup. Omitting profile selects the exact default or the selection's explicit profile.
Use the full 64-character projection digest printed by import, not the portable directory name.
The Python service opens a read-only Rust repository and closes it at lifespan exit. It retains
native/lexical state, with relational hydration and vector ranks fetched from PostgreSQL.

Import validates source before opening a lease, freezes transport/batch identity, copies at most
1,000 rows/16 MiB per transaction, and commits rows with their receipts. A generation advisory
lock serializes retries. Interrupted loading/validation is resumable; conflicting content fails.
Stored rows, complete support closure, artifact bytes and required indexes must pass before one
ready transaction. Import/reconcile never select. Existing readers remain pinned after selection.
`serving cleanup --generation DIGEST` removes only inactive unpublished rows/artifact locations;
it keeps terminal metadata, attempts, receipts and content-addressed files. Ready generations and
artifacts remain while the generation is current. Validated design-phase cutover removes the superseded runtime epoch.

Exact vector ranks enumerate every eligible member in each of four evidence families before
BM25 and reciprocal-rank fusion. Request-local selection is shared across channels and final
assembly. Ranking carries the actual winning unit and fragment, whose metadata is checked against
PostgreSQL; `get_evidence` expands its rendering and original sources. Bodies are not loaded to
validate rank references. A missing query embedding permits explicit lexical-only ranking. The current operator deployment
uses that route under the PR4 live-embedding waiver.

The old fixed-view ANN tooling is removed. Future ANN adoption requires current-family recall,
numerical and performance qualification under [ADR-0078](adr/0078-current-design-cutover.md).
No prior index or qualification record activates a current retrieval route.

For a coherent operational report, reserve one or two provider connections in a protected serving
config's existing total budget (`provider_connections < max_connections`, maximum six total):

```sh
target/release/lctx db report --store STORE --snapshot SNAPSHOT_ID \
  --generation FULL_GENERATION_DIGEST --serving-config REPORT_CONFIG --format json
```

The report verifies the canonical snapshot and summarizes its library and corpus releases once,
using the same library identity as bundle publication. It captures the declared mutable operational views in one
read-only repeatable-read transaction, then joins them to immutable generation data in DataFusion.
An unready projection still produces a canonical summary and diagnostics. Attempts/events are
explicitly scoped to store/compiler and can include other libraries. The report names its capture
snapshot/time, transfer rows/bytes, query/transfer time and Arrow conversion time. Provider reads
admit declared binary/text/bool/int64 schemas and a closed expression policy; generation-qualified
inner key joins can federate, outer joins and unsupported expressions stay local.

Budgets refuse explicitly: hydration 64 MiB decoded/8 MiB response; ranks 32 MiB; fusion 128 MiB;
selection 200,000 rows per relation and two million indexed visits/witness elements; mutable report capture 100,000 rows/64 MiB/30 seconds. Native
work has two slots, one-second admission and a whole MCP request deadline of 30 seconds. Cancelled
native jobs retain slots until completion. Cancelled SQLx work retains its lease through wire drain
and the configured server statement deadline; uncertain drain closes the pool. The provider sends
CancelRequest and drains before releasing capacity, quarantining an unconfirmed slot. A cancellation
response does not claim instantaneous server/CPU termination.

`just pilot STORE LOG [SERVING_CONFIG] [PROFILE]` and `pilot-live` explicitly import the produced bundle
and smoke-test that pinned PostgreSQL generation without selecting it. Their configured database
must already be migrated. `PROFILE` defaults to `catalog`; `behavioral` exercises retained enrichment. Offline scoring references can be read without PostgreSQL, while online
ranking/structured evaluation use `--config` and the same pinned repository. `just test-postgres`
builds its own canonical fixture. `just test-postgres-reference PROJECTION REFERENCE_JSON` runs
the separate saved-answer parity comparison; retained inputs and commands are in the
[PG12–PG15 evidence](design_review/evidence/2026-09-28_postgresql-query/README.md).


### Original evidence and deployment observations

**Implemented and bounded Tested, 2026-09-28 (ADR-0076/0077).** The current contract retains original Binary/bytea artifacts, contextual scenarios, deployment
observations and typed site associations. Delta remains canonical. `get_operation` returns
bounded evidence references; `get_evidence` pages original source. Its `metadata_omitted` flag
reports oversized detail while preserving primary-source access. No source executes during reads.

Run task observations explicitly before compiling with their receipts:

```sh
uv run python scripts/deployment_check.py --source SOURCE_CHECKOUT --out build/task-checks
target/release/lctx compile fastmcp --profile catalog --store STORE \
  --evidence-observations build/task-checks/programmatic.json \
  --evidence-observations build/task-checks/cli.json
```

The fixed policy uses the pinned upstream `examples/fastmcp_config/server.py`, a disposable locked
environment, and real MCP list/add interactions for programmatic and CLI-entry-point launch.
Receipts bind source, runner, interpreter/runtime, lock and metadata digests, exact commands,
inputs and observed outcomes. A changed runner/environment/source requires fresh observations;
failed tasks remain explicit failures. An all-extras task pass does not prove a minimal install.
No wheel is built: product native modules remain editable fastdev artifacts.

**Tested operator cutover, 2026-09-28.** Both current profiles passed real PG/MCP, catalog parity
and current reconstruction with embeddings disabled. The database contains exactly two current
ready generations. Behavioral is selected:

- Catalog: `bbbac7c4d106e487b47d33b2618d1ec47636905fee456d11157992e5be0ea7b8`.
- Behavioral: `b87c881a3838e21ddf1399089631d2472cc4d476094b343059c44cec5d463630`.

`build/store` and `build/generations` point to the current behavioral publication. Obsolete stores,
generations, runtime copies, rollback dumps and upgrade credentials were removed after validation.
The [PR4 evidence](design_review/evidence/2026-09-28_pr4/README.md) owns the exact receipts.

Two enriched operation packets (`fastmcp.cli.cli.run` and `fastmcp.server.auth.JWTVerifier`) exceed
the 256 KiB expanded budget and explicitly refuse; their catalog-profile packets fit. Their winning
units still expose complete original bytes through `get_evidence`. PR5 owns any optional enrichment
pagination/omission design; the current path neither truncates nor relabels that refusal.

### Complete recovery and diagnostics

`serving status [--generation DIGEST] [--verify-artifacts]` is a bounded, redacted read-only
observation that also reports unavailable databases and incompatible schemas. It separates
publication, artifact availability, selections and profile admission. Startup logs only the pin;
optional diagnostic queries do not become serving prerequisites. The importer configuration
selects the database; `--verify-artifacts` explicitly hashes retained files.

Backup receipt format 3 (inventory2) uses one exported snapshot for pg_dump, streaming logical-root table
fingerprints, ready manifests and selections. It copies each required native/lexical artifact into
`ARCHIVE.artifacts/SHA256/NAME`. The completion receipt appears only after checksums and fsync.
Keep a current-format dump and its matching receipt/artifact directory together while exercising
its restoration. Current reconstruction verifies all advertised relations and original bytes,
reconciles locations and serves the selected generation under the exact profile. Older formats
are rejected; there is no retained-runtime fallback. Once a validated replacement is active,
remove obsolete backup and runtime artifacts under ADR-0078.
