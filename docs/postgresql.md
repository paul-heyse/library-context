# PostgreSQL operations

**Implemented and Tested, 2026-09-28; qualification receipts:**
[PG16–PG17 operations evidence](design_review/evidence/2026-09-28_postgresql-operations/README.md).
[ADR-0068](adr/0068-postgresql-serving-and-standard-embeddings.md) owns the SQLx boundary;
[the plan](plans/postgresql-integration-plan_2026-09-27.md) records initial acceptance and
owns conditional capabilities. These instructions concern one operator's local application database.

## Local deployment and configuration

The installed `18/main` PostgreSQL 18.6 cluster on port 5432 hosts database `lctx`.
`lctx_migrator` owns the database/migrations; `lctx_app` is a separate limited login.
The bootstrap leaves host authentication unchanged and uses loopback SCRAM. Neither login has
superuser, createdb, createrole or replication privileges. `16/main` and unrelated databases
are outside this application deployment. pgvector 0.8.6 is installed explicitly in the protected `lctx_ext` schema.

For a new deployment, run `uv run python scripts/postgres_bootstrap.py` interactively once.
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

## PG8–PG11 deployment and rollback

**Implemented and deployed through PG17, 2026-09-28.** After the PG7 backup,
`uv run python scripts/postgres_expand.py` installs pinned pgvector and provisions `lctx_importer`
and `lctx_serving`. It does not migrate schemas. Run the newly built `target/release/lctx db migrate`
then `target/release/lctx db check`. Application/serving startup only checks compatibility.

Protected files in `~/.config/library-context/` now separate capabilities:

| File | Capability |
|---|---|
| `postgres.json` | Existing cache/operations application connection and limits |
| `postgres-admin.json` | Migration connection only; explicit commands and protected backup tooling |
| `postgres-importer.json` | At most two import connections; TEMP staging allowed, application schema DDL denied |
| `postgres-serving.json` | Read-only role; six total connections by default, zero provider connections until selected |

All are regular mode-0600 files. The pending expansion file preserves generated credentials for
idempotent recovery and is also protected. Never copy credentials into command lines, logs or
reports. Both clients use `pg_catalog,lctx_ext` as their trusted search path. Remote connections
require hostname-verifying TLS; a provider budget is reserved from the total, not added to it.
Runtime statements/locks/acquisition and cancelled leases are bounded. `lctx_storage` opens only
when explicitly awaited and closes at lifespan exit; importing the module makes no connection.

The SQLx migration history now lives in `crates/lctx-postgres/migrations`; the original two
migrations are byte-preserved. Projection schema installation does not publish a ready generation.
PG12 owns load/freeze/validation/promotion and child vector partition creation. Import uses TEMP
COPY followed by generation-qualified inserts: RLS protects final tables. No ready-generation
cleanup is available. Native/lexical artifacts remain digest-checked files with manifest references.

The preserved PG7 binary, locks, config and dump are under
`build/postgresql-pg8-baseline-6403b60/`. Its old reader/schema check cannot use the upgraded database.
Rollback requires restoring the PG7 dump into a separate database and pairing that endpoint/config
with the old binary, FORMAT 10 generation and spec. Do not reverse-migrate canonical receipts.
`postgres_backup.py restore-drill` chooses the receipt's pinned image and compares protected
snapshot fingerprints; PG7 receipts retain their original image and tables.

## PG12–PG15 publication and query operations

**Implemented and deployed, 2026-09-28.** PG17 completed migrations001–008; product PR1/PR2 add009/010.
The live 1024 FastMCP generation is selected with exact retrieval; no ANN class met its calibration
benefit/plan gates. Serving config reserves two of its six connections for the provider and four
for SQLx; importer has two, and the application cache pool has six. Budgets are per process.
No read, import or MCP startup migrates schemas automatically.

The protected populated `build/postgresql-pg17-operator.dump` receipt/artifact set restored both
ready generations and the captured exact selection through least-privilege native/MCP serving
in 15.31 seconds (900-second local objective). Retain the matching prior schema004/schema007
binary, native/Python sources, config, artifacts and dumps in
`build/postgresql-pg16-baseline-9471b93/`, as well as the PG7 baseline
`build/postgresql-pg8-baseline-6403b60/`. Never pair a rollback database with a newer runtime. Daily backup remains an operator action; no scheduler was installed.

Current projection FORMAT4 / bundle FORMAT14 adds surface/configuration/field-link records
(ADR-0073/0074), retaining release/coverage context and explicit catalog/behavioral capabilities
from ADR-0072. Retained PG17 and PR1 generations use FORMAT2/12 and FORMAT3/13.
Re-export compatible canonical snapshots with `lctx bundle`; this does not re-embed their vectors.
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
uv run lctx-mcp --library fastmcp --embedder vllm
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
artifacts are retained. No automated pruning is installed.

Exact vector ranks enumerate all eligible entities and both operation views before BM25/RRF
fusion. HNSW indexes alone change no route. To prepare optional ANN qualification:

```sh
target/release/lctx serving build-hnsw --generation FULL_GENERATION_DIGEST
uv run python scripts/postgres_qualification_pack.py BUNDLE_DIRECTORY REQUESTS_JSON \
  --out FROZEN_PACK_JSON --config SERVING_CONFIG --embedder vllm \
  --phase calibration --maximum-ann-p95-ms 250
target/release/lctx serving qualify-hnsw --pack FROZEN_PACK_JSON \
  --serving-config SERVING_CONFIG
# Retain the calibration JSON stdout. Freeze disjoint requests, then confirm:
uv run python scripts/postgres_qualification_pack.py BUNDLE_DIRECTORY CONFIRMATION_REQUESTS_JSON \
  --out CONFIRMATION_PACK_JSON --config SERVING_CONFIG --embedder vllm \
  --phase confirmation --calibration CALIBRATION_REPORT_JSON
target/release/lctx serving qualify-hnsw --pack CONFIRMATION_PACK_JSON \
  --serving-config SERVING_CONFIG
# Only after both confirmation runs pass for the current physical indexes:
target/release/lctx serving select --library fastmcp --generation FULL_GENERATION_DIGEST \
  --profile QUALIFIED_PROFILE_DIGEST
```

Mixed format 2 always routes briefs, <=1024/4096 eligible vector-bearing operations and <=10%
selectivity to exact. Calibration selects the smallest passing count floor and independently
admitted broad/unfiltered classes. Confirmation uses disjoint queries and cannot retune policy.
Each class needs eight distinct requests and two paired runs (three warmups, ten repetitions),
99% mean recall@10 per active view and fused result, natural custom/generic index plans, no
underfill fallback, <=250 ms rank-stage p95 and at least 20% improvement over exact. Timing
includes routing, admission catalog work and compact IPC materialization under the serving role.
Embedding and MCP protocol latency are reported separately. Comparison revision 2 checks
complete inventories, finite per-entity scores within 1e-5 and each calculation's score/ID order;
float64 ordinal crossings and top-10/fusion differences are reported separately. This preserves
the declared PostgreSQL numerical policy without fuzzy ties. Frozen packs remain <=64 cases/128 MiB.

Failed calibration/confirmation records evidence without selecting a profile. Exact-only deployment
is supported when ANN has no measured benefit. Reindex, restore, index replacement, server/extension
or retrieval-engine changes invalidate physical admission: explicit profile pins and ANN queries
refuse stale admission. Requalify the actual indexes or explicitly select exact. Old format-1 ANN
attempts remain historical evidence and cannot bypass this admission boundary.

For a coherent operational report, reserve one or two provider connections in a protected serving
config's existing total budget (`provider_connections < max_connections`, maximum six total):

```sh
target/release/lctx db report --store STORE --snapshot SNAPSHOT_ID \
  --generation FULL_GENERATION_DIGEST --serving-config REPORT_CONFIG --format json
```

The report verifies the canonical snapshot and summarizes its library and corpus releases once,
using the same library identity as bundle publication. It captures seven mutable operational views in one
read-only repeatable-read transaction, then joins them to immutable generation data in DataFusion.
An unready projection still produces a canonical summary and diagnostics. Attempts/events are
explicitly scoped to store/compiler and can include other libraries. The report names its capture
snapshot/time, transfer rows/bytes, query/transfer time and Arrow conversion time. Provider reads
admit declared binary/text/bool/int64 schemas and a closed expression policy; generation-qualified
inner key joins can federate, outer joins and unsupported expressions stay local.

Budgets refuse explicitly: hydration 64 MiB decoded/8 MiB response; ranks 32 MiB; fusion 128 MiB;
ANN rescoring 200,000 rows/128 MiB; mutable report capture 100,000 rows/64 MiB/30 seconds. Native
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

**Implemented; PR3 qualification in progress, 2026-09-28 (ADR-0076).** Bundle15/projection5,
wire2 and migration011 add original Binary/bytea artifacts, contextual scenarios, deployment
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

The schema010 source/runtime/protected backup is retained under `build/pr3-baseline/`; its
matching-runtime restore passed. Migration011 and current operator selection are tracked in
[PR3 qualification](design_review/evidence/2026-09-28_pr3/README.md).

### Current PR3 catalog deployment

**Implemented and Tested, 2026-09-28:** explicit `lctx db migrate`/`db check` applied additive
migration011. Both final live FastMCP4.0.5 profiles imported, reconciled and served through stdio MCP
using the editable runtime, including original evidence and both current SHA-bound task observations.
The selected behavioral generation is `d874d3694612e12d99272f699fe474e5cce9758cb5cf860dd49cbd52fccd58b2`,
with exact profile `ff645e4a55461d3041fa4dbbd56f01d90da07b74a55040ec350933ac23ef4586`.
Catalog-only `0b2fe1205720f3291853671629935afc8137f968437f7a0be66e5be81effce74` is ready. All six older ready generations and artifacts remain retained.

`build/pr3-operator-cutover/current.dump` plus protected receipt/artifacts contains 86 tables and
eight ready generations. Its disposable restore passed in 28.59 s, preserving legacy content,
serving both current profiles and restoring the exact behavioral selection. The schema010 baseline
at `build/pr3-baseline/` preserves source `5aebab4`, reconstructed matching CLI, saved native modules,
runtime checksums and protected dump/receipt/artifacts. Its matching-runtime restore passed in 21.41 s.

Keep PR2/PR1 populated dumps and matching baseline runtimes, alongside the older PG recovery assets.
Use each runtime with its matching restored schema; do not reverse migrations or relax reader checks.
The temporary controlled embedding service stopped after live operator checks. Start `just embed-serve`
for live query embeddings. [PR3 evidence](design_review/evidence/2026-09-28_pr3/README.md) owns receipts
and qualification limits. General evidence search/browse and classifier/journey work remain PR4–PR5.

### Complete recovery and diagnostics

`serving status [--generation DIGEST] [--verify-artifacts]` is a bounded, redacted read-only
observation that also reports unavailable databases and incompatible schemas. It separates
publication, artifact availability, selections and profile admission. Startup logs only the pin;
optional diagnostic queries do not become serving prerequisites. The importer configuration
selects the database; `--verify-artifacts` explicitly hashes retained files.

Backup receipt format 3 (inventory2) uses one exported snapshot for pg_dump, streaming logical-root table
fingerprints, ready manifests and selections. It copies each required native/lexical artifact into
`ARCHIVE.artifacts/SHA256/NAME`. The completion receipt appears only after checksums and fsync.
Keep the dump, JSON receipt and artifact directory together. Incomplete output is retained for
inspection and is not a recovery point. Daily backups remain an operator responsibility.

`postgres_backup.py restore-drill ARCHIVE` creates only an owned disposable PG18/vector database.
It compares all logical rows and the receipt's complete inventory before mutations; verifies
sequences/event writes; relocates every retained artifact with old paths unavailable; reconciles and
serves current bundle15 catalog and behavioral generations; and restores the recorded selection with
an explicit exact profile. Retained bundle12/13/14 rows, exact manifest bytes and artifacts are verified as
preserved with `legacy_runtime_required`; they are not admitted to the new reader. An incompatible
selected pointer fails usable-serving recovery rather than choosing another generation. The 15-minute RTO includes verification and usable serving.
Logical restore never inherits ANN admission. A compatible historical receipt uses the retained
legacy restore verifier; schema008 receipt2 requires the retained pre-PR1 recovery scripts and
matching binary/Python/native runtime against a separately restored database. Never bypass migration
checks to pair the old runtime with schema009. The PR1 baseline is retained under
`build/postgresql-pr1-baseline-a6c9fcf/`; rollback captures its pre-cutover history only.

Finite index/analyze maintenance uses a 300-second limit, 256 MiB maintenance memory and two
parallel workers. Normal serving limits stay at 30 seconds. Retain all ready generations and
artifacts; observe vector partition/index bytes, backup bytes and cluster-wide WAL growth before
manual capacity changes. Automatic deletion, replicas and PITR remain later F10 scope.
