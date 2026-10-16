# Pinned library evidence for the runtime review

**Interface-checked, 2026-10-16.** Supporting evidence for the
[principal review](../../reviews/design_review_catalog-runtime-efficiency-target_2026-10-16.md),
not a separate architectural verdict. No builds, tests or database/service calls were performed.
Library research used selected capability skills, exact local source, locked dependency identities
and Context7 resolve/query documentation. Current documentation is a discovery lead where it
describes a newer release; it does not change the pinned contract.

## Native engine and default installation

The default installation record at
`/home/paul/.local/state/library-context/surrealdb/installation.json` identifies installation
`58abe9ea-2088-4109-95ab-8440a3749c51` and selected tool generation
`9fbab612b96f62d2e88070ffd2cb7f1d8ec7b7a3a5cf304592c1c041ed3c761e`.
Its tool record points to server source
`/home/paul/.local/state/library-context/surrealdb.tools/server-build/cb1f97ec9048566bca79c9f4f5ad3584a18ffc4cb39c31db13a5450f5605bd4b/source `.
That checkout's HEAD is `238bfeb11f5725bebed370167656748df8067595`; the patched lock's SHA256 is
`d692b2e71fa7402e4171ad7c19912fc931d699b665c3ed168d95f8a9b26a9e2a`.
It resolves `surrealdb-rocksdb 0.24.0-surreal.5` and
`surrealdb-librocksdb-sys 0.18.3+11.0.0-4`. The upstream backend manifest's `.4` requirement is
not the resolved version. The root product lock is a patched remote SDK 3.3.0 lock; it does not
contain the server core/KVS/RocksDB dependency closure.

An initial inquiry checked the disposed E1 campaign's tool source. Applicability was then
independently rechecked against the default installation above. The four memory/default files
are byte-identical between those inspected sources and unchanged against their upstream HEAD.
Only the corrected default-target evidence supports the following build-source claims:

| Contract | Exact source and limitation |
|---|---|
| RocksDB cache default | [`cnf.rs`](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/kvs-rocksdb/src/cnf.rs): `max(detected_memory / 2 - 1 GiB, 16 MiB)`. This is a default, not measured occupancy or a whole-process RSS limit. |
| Memory detection | [`sys.rs`](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/common/src/sys.rs): positive cgroup limit when available, otherwise host total, with a 1 GiB fallback for zero reported total. The daemon's actual detected value was not observed. |
| Threshold default | [`cnf/src/lib.rs`](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/cnf/src/lib.rs): absent `SURREAL_MEMORY_THRESHOLD` returns zero, disabling that guard. Effective process overrides were not read. |
| Memory-accounting scope | [`memory_manager.rs`](https://github.com/surrealdb/surrealdb/blob/238bfeb11f5725bebed370167656748df8067595/surrealdb/kvs-rocksdb/src/memory_manager.rs): shared cache includes block/row/blob cache and memtable reservations; transaction write batches, SST readers, compaction/flush and readahead are outside that accounting. |
| Newer running-query intervention | [Current documentation](https://github.com/surrealdb/docs.surrealdb.com/blob/main/src/content/reference/cli/db/environment-variables.mdx), fetched through Context7, marks running-query memory-threshold intervention as 3.3.1. Do not transfer it to the pinned 3.3.0 server. |

These defaults make warm cache a plausible contributor to the earlier approximately 143 GB RSS
observation. They establish neither cache residency nor a leak. Existing client ResourceBudget
reservations and query/finality observations do not partition server heap bytes. No process
environment, credentials, cache statistics or live query state were read in this inquiry.

Bulk ordinary/relation insertion is already used by canonical immutable installation. Equality
checks and independent ownership remain application obligations. `INSERT IGNORE` used for a
disposable embedding cache is not interchangeable with canonical collision refusal. Native
retirement's existing scalar `SELECT ... FROM ONLY $id FOR UPDATE` reads preserve the streaming
executor's locking behavior; a record-ID-array query is not a qualified bulk-lock replacement.
RocksDB `getm`/`multi_get_opt` availability alone does not establish a particular SurrealQL plan.

## Compute, preparation and task ownership

Locked versions inspected: DataFusion 55.1.0, Arrow 59.3.0, Tokio 1.53.2, petgraph 0.8.3,
leiden-rs 0.8.1 and biodivine-lib-bdd 0.6.3. Exact versions come from Cargo manifests/lock;
the SDK/server backend identities above have different dependency scopes.

- Existing attempt-owned DataFusion runtime, allocation pool and spill directory already support
  shared preparation. Immutable provider catalogs are also reused. Reusing logical preparation
  must retain exact providers and dependencies; physical plan reuse additionally requires
  [state reset](https://docs.rs/datafusion-physical-plan/55.1.0/datafusion_physical_plan/execution_plan/fn.reset_plan_states.html),
  with dynamic-filter/recursive-plan limitations. It is not a universal drop-in result cache.
- DataFusion's ordinary `cache()` collects into a MemTable. It is not a spill-backed replacement
  for the existing captured-input owner. Streams avoid final collection, while blocking operators
  can still retain state. Root batch discovery and typed partition views already provide a
  suitable mechanism for sharing overlapping Base inputs without another materialization layer.
- [Tokio 1.53.2 `block_in_place`](https://docs.rs/tokio/1.53.2/tokio/task/fn.block_in_place.html)
  suspends other futures in the same task while allowing independently spawned tasks to progress.
  This was fetched through Context7 and confirmed in exact local `src/task/blocking.rs:14-18`.
  Thus a parent FuturesUnordered does not independently schedule synchronous child kernels.
- [Tokio 1.53.2 `spawn_blocking`](https://docs.rs/tokio/1.53.2/tokio/task/fn.spawn_blocking.html)
  cannot abort a started closure. Runtime shutdown timeout stops waiting without stopping that
  work. An independently scheduled CPU owner therefore needs cooperative cancellation where
  required and an actual join before input/charge release; dropping or aborting a handle is not
  drainage. Exact local source `src/task/blocking.rs:104-119` confirms the fetched contract.
- Selected graph hydration, petgraph SCCs, indexed proof buckets, delta-style enqueueing and
  Pareto representatives already serve their declared consumers. The inquiry found no basis for
  replacing them with database reachability, another graph backend or an incremental framework.

All performance benefits remain unmeasured. Built-in availability qualifies an alternative;
its semantic fit, retained owners and whole-operation burden still determine the recommendation.
