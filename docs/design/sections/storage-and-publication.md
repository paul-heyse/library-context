# Storage and publication

**Implemented / Tested for facts and normalized storage, 2026-09-30; Phase 4 qualification in
progress, 2026-10-01.** [§15](semantic-model.md) owns declarations, immutable vocabulary epochs,
semantic validation and generation admission. PostgreSQL is the single canonical relational store;
DataFusion computes over generation-bound providers. The [cutover plan](../../plans/semantic-model-cutover-plan_2026-09-29.md)
owns current qualification. Serving remains unavailable pending Phase 5.

## §5 Projections

**Implemented / focused-Tested, 2026-09-30.** Model-owned projection declarations specify input,
context, universe, vertices, arcs, admission policy and exact lineage. `cpg-extract` publishes the
normalized program projections; `cpg-core::analysis_graphs` hydrates each stored graph once within
the attempt budget. Algorithms borrow a branded graph view through callback-bounded permits;
they cannot reuse a token for another projection or outlive its consumer access.

Parallel arcs, isolates, direction and policy boundaries are preserved. A selector limits results,
never the graph universe. SCC partitions and graph fingerprints are canonical. Missing graph
permits, foreign budgets, incomplete source receipts and invalid endpoints refuse. Graph kernels
and their shared semantic contracts are in [§15.10](semantic-model.md#section-15-10) and
[§9](analytics.md#section-9). No hydration-cost, pilot-scale or total-RSS measurement is claimed.

> Decision: ADR-0086, ADR-0103, ADR-0044, ADR-0085

## §6 Persistence and publication

`lctx-postgres` owns generation tables, roles, COPY, publication and leases. `cpg-core` owns the
DataFusion provider/runtime adapter. Pure model operations perform no I/O and are the same
validators used by tests and publication. [§15.11](semantic-model.md#section-15-11) specifies the
closed-epoch protocol; [the runbook](../../postgresql.md) owns operator commands.

> Decision: ADR-0086

### §6.1 Canonical tables and publication

**Implemented / Tested at the facts frontier, 2026-09-30.** An attempt stages a self-contained
captured input and declared results. Typed records derive identity, Arrow/PostgreSQL lowerings,
references, COPY codecs and shared invariants. A completed ordinary stage has immutable outputs
and acknowledged receipts. Vocabulary contributions remain private until their publication group
closes atomically with the result relations that reference them.

Group closure drains readers/writers, merges contributions, deduplicates identical payloads,
validates exact source membership and revokes temporary grants. Existing vocabulary payloads and
old receipt meanings never change. Failed, cancelled or unconfirmed closure poisons the attempt;
no partial publication becomes available. Final seal requires every group closed and frontier
admission satisfied. Compilation never selects the published generation.

> Decision: ADR-0086, ADR-0105, ADR-0108, ADR-0126

### §6.2 Readers

**Implemented / focused-Tested, 2026-09-30.** Completed-stage capabilities and published readers
are separate authorities. `AttemptSession` reads only acknowledged predecessors at declared
vocabulary prefixes. `GenerationSession` pins one admitted immutable generation. Provider pools,
read leases and query permits live through stream drain; the full physical plan must fit declared
scan capacity before scanning. Read-only SQL rejects writes and typed parameter binding preserves
values. Missing relations and required-null payloads refuse rather than producing empty answers.

A later vocabulary prefix cannot enlarge reconstruction of an earlier normalized owner. Shared
invariants name the original Facts prefix explicitly; no consumer invents a second source inventory.

> Decision: ADR-0086, ADR-0094, ADR-0105

**Implemented / focused-Tested trust boundary, 2026-10-05.** Complete acknowledgements
are trusted under enforced owned immutability. Direct output locks drain in-flight writes before
receipt/grant transitions. Private vocabulary deltas are drained, sealed and revoked before merge;
closed prefix payloads, membership and view meaning remain immutable under legal service operations.
Routine reads resolve acknowledged identities and current admission without privileged-tamper
rehashing. `lctx generation audit` is explicit, budgeted and read-only; repair quiesces consumers and
rebuilds under a new installation/generation identity. Candidate proof writes commit atomically with
closure; failure/cancellation/uncertain acknowledgement supplies no usable conclusion. No global
proof cache or compatibility receipt reader is introduced. Qualification belongs to the
[assurance coordinator](../../plans/testing-architecture-pivot-plan_2026-10-04.md).

An admitted checkpoint atomically retains exact digest/count acknowledgements for its covered
ordinary frames, including frozen empty relations without a producer in the selected profile.
Reuse requires the matching frontier contract, model, schedule, coverage, content, physical layout,
profile and canonical physical frame. Vocabulary remains bound to its own closed-prefix receipts.
Checkpoint acknowledgement supplies neither a completed producer nor an undeclared source grant;
both authorities remain independently required. Referring relations select validation definitions;
checkpoint-qualified premises complete their inputs without selecting unrelated checks. Vocabulary
closure binds that existing admitted checkpoint, resolves checkpoint-only inputs before candidate
execution and retains vocabulary prefixes independently. Failed or rolled-back checkpoint work leaves no
usable frame acknowledgement. Disposable PostgreSQL controls exercise writer draining/revocation,
rollback, identity misses, receipt reuse and admission-reservation release.

> Decision: ADR-0126

### §6.3 Schema evolution

**Implemented, 2026-09-30.** Executable declarations are the schema authority. Contract snapshots
are checked with updates disabled; accepting a changed snapshot is a schema migration. Codebooks
are append-only. Recollect changed pinned input into a fresh generation: no old-format reader,
legacy-ID bridge, compatibility adapter, dual store or incremental semantic repair is retained.
Named current readers are quiesced before replacing their state.

> Decision: ADR-0086, ADR-0087, ADR-0048

### §6.4 Serving generations

**Accepted Phase 5 target; unavailable, 2026-10-01.** Serving pins the canonical generation through
model-generated views, grants and indexes. It does not import a bundle into another canonical
store. Native/Python wire, cursor, hydration, journey and dormant `cpg-core::bundle` controls are
retained reconstruction expectations. Derived artifact caches must preserve exact identity and
cannot supply semantic authority. [§15.12](semantic-model.md#section-15-12) and
[§11.3](synthesis-and-serving.md#section-11-3) own the remaining serving contract.

> Decision: ADR-0086, ADR-0087, ADR-0049, ADR-0071, ADR-0077, ADR-0078

<a id="section-6-5"></a>

### §6.5 PostgreSQL services and capability adoption

**Implemented and bounded Tested, 2026-09-28; live embedding waived for PR4.** The
[PostgreSQL workstream](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-workstream)
owns deployment and qualification. The [forward plan §6](../../plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-findings)
owns review dispositions. [Operating instructions](../../postgresql.md) cover the deployed service.

**Owners and authority.** `lctx-postgres` owns the retained embedding cache and operation
services, PostgreSQL generation storage, roles and migrations. `cpg-core` coordinates typed
compilation and bounded DataFusion generation reads; `lctx` owns operator commands. These effects
use SQLx with the existing Tokio runtime and one Rustls configuration. Semantic definitions and
validation remain model-owned.

**Cache and replay (Implemented; Phase 4 qualification in progress, 2026-10-01).** The shared
cache commits one insert-only winning value per full spec/input key. An attempt retains the exact
winning bytes it consumes in model-owned `embedding` relations, with explicit Analytic and
Retrieval consumer records. Canonical value codecs and content identity belong to `lctx-model`.
Reconstruction uses those immutable generation inputs; it does not contact the cache or embedder.
The fake-embedding controls qualify cache reuse and replay, not live-service usability or quality.

**Operation services (Implemented, bounded Tested, 2026-09-28).** Attempt history and operational
records retain their migration, backup/restore and reconciliation contracts. Generation discovery
now reads the canonical generation store. Operational service rows cannot publish a semantic
generation or strengthen a verdict. Rebuildable analytical data follows §6.3.

**Serving and hydration (Accepted Phase 5 target; unavailable, 2026-10-01).** Bound query values,
nominal ID conversion, generation membership, required-null refusal, bounded rows/bytes, stream
leases and typed response assembly remain required. Dormant native/Python response, cursor and
hydration controls are preserved as reconstruction expectations. Earlier FORMAT manifests and
serving-projection receipts qualify the retired pipeline only; they are not current schema or
admission authority. Model-generated views and codecs replace independent mappings (§15.12).

**Deployment boundary.** Reuse PG18 with pinned pgvector 0.8.6 in the locked `lctx_ext` schema.
Application, importer, serving and migration credentials/grants are separate. Keep connection pools,
SQL operations, retries, transaction lifetime and memory/disk/WAL use bounded. Embedding/network
work runs outside database transactions. Runtime configuration and secrets are explicit and
redacted; checked query builds force `SQLX_OFFLINE=true`. Migrations are an explicit command,
not a query/startup side effect. Disposable tests assert PG18 and use an explicitly pinned
image; source inspection of libraries does not establish deployment compatibility.

**Capability map.** Retained operational services are implemented; generation serving and
conditional later consumers remain distinct. Installed versions are in `docs/pins.md`; conditional candidates remain in the implementation plan.

| Capability / library mechanism | Initial or later consumer | Contract and adoption boundary |
|---|---|---|
| SQLx checked SQL/files, typed rows, runtime queries and statement cache | Initial cache/operations; later projections | Offline metadata follows one migrated PG18 schema; dynamic queries and overrides have executed controls; domain validation remains shared |
| SQLx pool, transaction/isolation/savepoint APIs, error codes, Rustls | Initial application effect modules | Bounded leases/deadlines, explicit SQLSTATE retry policy, no connection across external I/O; selected root/hostname policy |
| SQLx SQL migrations and locking | Initial application schema lifecycle | One history and owner; fresh/upgrade/failure/compatibility checks; runtime role has no DDL authority |
| SQLx streaming, batching and raw COPY | Bounded initial transfers; larger imports when measured | Backpressure, byte/type fidelity and cancellation/reuse controls; compare established typed COPY/Arrow routes before bespoke generic encoding |
| PostgreSQL constraints, native types, JSONB/arrays, indexes, CTEs/windows and partitioning | Schema/query-specific operational or projection consumer | Use through ordinary SQLx SQL when it earns its cost; compiler IDs/codebooks remain derived mappings, not independently authored meanings |
| SeaQuery + sea-query-sqlx | Later variable joins/expressions | One typed structural query owner and binder; values bound; no duplicate Stage 4 concept-definition semantics |
| SQLx PgListener / LISTEN-NOTIFY, row locks and work claiming | Later live operational consumer/job requirement | Notification is a wakeup; persistent events/rows determine state. Reconnect, idempotency, missed notification and crash cases before durable workflow claims |
| Testcontainers modules | Initial and later real PostgreSQL integration tests | Pin server version/digest and extension-bearing image where needed; actual application role and migrations; a missing daemon is a block |
| Psycopg 3 / psycopg_pool; possibly SQLAlchemy | Direct Python retrieval/operator domain only | Async lifetime/resource ownership, Python 3.14 qualification, one pool and one migration history; no Python semantic interpreter |
| pgvector; text/trigram remains conditional | Selected immutable vector projection, PG14 retrieval | §11.4 owns generation, evidence and ranking/approximation obligations; cache storage alone is no trigger |
| Owned DataFusion PostgreSQL provider; ADBC remains conditional | Selected PG15 read/federation consumer | Match pinned family and prove metadata/null/ID/value/ordering/pushdown/read-view semantics; no unsafe layout conversion |
| pgrx | Measured SQL-side filtering/aggregation can avoid material transfer | Separate PG18 extension deployment; shared pure kernel, backend lifetime/thread/cancel constraints and unchanged evidence/model meaning |
| pg_stat_statements / backup-WAL-replication / pooler | Observability or recovery/scale requirement | Explicit deployment and resource cost; extension preload/restart and proxy session semantics qualified before use |
| Cornucopia + Rust-Postgres family; ORMs | A different query/domain programming model earns lower total complexity | Revisit driver decision through ADR; alternatives are not layered onto the base speculatively |

Future review events preserve exact subject revision and become explicit attributed compile
inputs when used. Later SQL serving consumes immutable canonical generations. Both need their own
consumer, replay and failure evidence; neither is silently enabled by installing PostgreSQL.

> Decision: ADR-0078, ADR-0086, ADR-0073


**Current-only replacement contract (Accepted; ADR-0078).** Validate both current profiles and
current-format reconstruction, quiesce project readers/writers, replace the operator state, then
remove superseded stores, generations, runtime copies and backups. Old-format readers and mixed
exact/ANN routes are removed. The forward plan owns dated deployment completion and any remaining
qualification boundary; current backup/reconstruction controls do not establish historical recovery.
