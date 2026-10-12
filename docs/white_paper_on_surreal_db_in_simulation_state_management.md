# SurrealDB as a unified simulation-state and persistence platform

The strongest approach is to treat SurrealDB as one canonical persistence system with multiple, explicitly managed data lifecycles, rather than treating temporary and permanent information as fundamentally different storage problems.

For simulation tooling, I would consider combining five design patterns:

1. Lifecycle-aware data contracts — assign retention, mutability, and recovery semantics to records and artifacts.
2. Execution-scoped state — distinguish experiments, simulation runs, execution attempts, checkpoints, and test executions.
3. Transactional promotion — elevate temporary results to permanent records without unnecessary duplication.
4. Reference-aware garbage collection — automatically reclaim expired simulation state while protecting anything needed for reproducibility or resumption.
5. Database-native automation and observability — leverage SurrealDB's events, graph references, versioning, changefeeds, and management interfaces.

The important distinction is that the simulation's execution lifecycle and the data's retention lifecycle should be modeled independently.

A simulation can be paused but disposable, completed but temporary, failed but worth preserving, or still running while generating checkpoints that must survive process failure.

## 1. Establish a lifecycle taxonomy

I would use the following logical classifications, regardless of whether they ultimately map to separate tables or databases.

| Lifecycle class           | Typical contents                                                           | Retention behavior                                                                    |
| ------------------------- | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------- |
| Ephemeral scratch         | Solver iterations, trial values, temporary transformations                 | Automatically discard after run completion or expiration                              |
| Recoverable working state | Paused simulation conditions, intermediate checkpoints, solver warm starts | Retain until the execution is resumed, abandoned, or expires                          |
| Test artifacts            | Unit/integration test runs, fixtures, regression outputs                   | Short retention, except failures or promoted baselines                                |
| Completed results         | Validated simulation outputs and final conditions                          | Retain according to experiment/result policy                                          |
| Canonical reference state | Initial conditions, reusable scenarios, validation baselines               | Indefinite or explicitly versioned retention                                          |
| Audit/provenance          | Configuration, solver version, source revision, execution lineage          | Independently governed; frequently longer-lived than the underlying intermediate data |

These are logical categories, not necessarily six physical storage locations.

In particular, the same simulation state could transition from a recoverable checkpoint to a permanent reference state without changing its numerical content.

### Important current SurrealDB limitation

As of the current 3.x documentation, SurrealDB does not offer native record-level TTL or a built-in time scheduler. An expiration timestamp does not automatically delete a record.

SurrealDB's database events can react to record mutations, but time-based expiration still requires an external scheduled process or an application-owned lifecycle controller.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



That makes a relatively small lifecycle-management subsystem worthwhile. It need not introduce any additional persistence platform.

## 2. Choose the right physical separation strategy

SurrealDB offers three useful levels of isolation: namespaces, databases, and tables. The best architecture is likely a hybrid.

| Strategy                                | Advantages                                                     | Limitations                                            | Recommended use                                            |
| --------------------------------------- | -------------------------------------------------------------- | ------------------------------------------------------ | ---------------------------------------------------------- |
| Shared database, distinct tables        | Cross-lifecycle references, transactions, and atomic promotion | Requires explicit cleanup                              | Core simulation state and permanent results                |
| Shared tables, lifecycle metadata       | No duplication; a record can change retention status directly  | Retention policies must be consistently enforced       | Checkpoints and artifacts with changing retention          |
| Disposable database per execution group | Strong isolation; whole database can be discarded              | No direct cross-database transactions or graph queries | Test suites, temporary experiments, bulk scratch workloads |
| Separate namespace                      | Stronger administrative and permission isolation               | More coordination and configuration                    | Development versus persistent environments                 |

SurrealDB supports multiple databases in a namespace, each with independent schemas, indexes, and permissions.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



A particularly useful recent improvement is that SurrealDB 3.2 introduced asynchronous physical reclamation for `REMOVE DATABASE`. The database is removed from the catalog immediately, and a background reaper cleans up its underlying storage. This makes whole-database disposal substantially more practical for test workloads.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



My preferred layout would be:

SurrealDB instance

Single canonical database platform

Persistent simulation database

Canonical scenarios / inputs

Run and execution metadata

Active state / checkpoints

Results / reference artifacts

Retention policy / audit

Record-level lifecycle management; transactional promotion

Disposable test databases

Test suite A

Isolated fixtures and outputs

Test suite B

Isolated fixtures and outputs

Temporary experiments

Separate bulk execution workspaces

Drop after completion

Persist only selected test summaries and failures to the canonical database

This preserves the ability to transition a resumable simulation into a permanent result without moving it between databases, while giving destructive testing an independent disposal boundary.

## 3. Model simulation persistence around immutable checkpoints

One potentially high-value change is separating the currently executing simulation from its saved state.

I would use a small set of canonical entities:

| Entity             | Ownership and purpose                                | Mutability                          |
| ------------------ | ---------------------------------------------------- | ----------------------------------- |
| `scenario`         | Reusable initial conditions, model and configuration | Immutable versioned revisions       |
| `simulation_run`   | Logical simulation execution and result lineage      | Mutable lifecycle metadata          |
| `run_attempt`      | A specific process invocation, restart, or retry     | Mutable until terminal              |
| `checkpoint`       | Restartable snapshot at an execution boundary        | Immutable once committed            |
| `state_chunk`      | Numerical state payload referenced by checkpoints    | Immutable; potentially deduplicated |
| `result_manifest`  | Published set of validated result artifacts          | Immutable after finalization        |
| `test_execution`   | Test outcome and associated diagnostic references    | Mutable until completion            |
| `retention_policy` | Centralized expiration and protection rules          | Versioned policy                    |

### The critical pattern: promote references, not payloads

Run attempt

Mutable execution cursor

Checkpoint manifest

Immutable, validated, restartable state

Immutable state chunks

Persist numerical state once

Resume simulation

Read checkpoint and update run cursor

Publish result

Create permanent manifest referencing existing chunks

The saved numerical payload is not intrinsically scratch or permanent. Its retention follows from its references, protection status, and lifecycle policy.

For example, a completed simulation could retain only its final checkpoint and selected intermediate milestones, while a paused simulation keeps all chunks necessary to resume. A failed test could preserve its failure-inducing initial state and diagnostics without retaining every transient solver iteration.

Why this matters: immutability makes checkpoint reuse, validation, deduplication, promotion, and garbage collection much easier to reason about. It also avoids treating the mutable `simulation_run` record as the full historical state of the simulation.

SurrealDB's transactions and graph/reference capabilities support this model directly. In version 3.3, `SELECT ... FOR UPDATE` also permits commit-time conflict checking for records read as part of a state transition.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+2



## 4. Build a small, reference-aware retention controller

This is the most important missing capability to supplement SurrealDB itself.

Rather than introducing Redis, a separate job database, or an orchestration platform with its own persistent state, I would use SurrealDB-backed retention records plus a lightweight scheduled Rust process.

The controller can own expiration, cleanup, checkpoint protection, and abandoned-run recovery.

### Suggested lifecycle mechanics

| Mechanism        | Implementation                               | Value                                           |
| ---------------- | -------------------------------------------- | ----------------------------------------------- |
| Expiration       | `expires_at` field with an index             | Efficiently locate eligible records             |
| Retention policy | `retention_policy` records                   | Centralized lifecycle rules                     |
| Protection       | Permanent references, pins, protected status | Prevent unintended deletion                     |
| Liveness         | Heartbeats and execution leases              | Distinguish active runs from abandoned ones     |
| Deletion         | Bounded, repeatable cleanup transactions     | Avoid massive deletion operations               |
| Orphan recovery  | Reference traversal or reference counts      | Reclaim abandoned chunks                        |
| Reconciliation   | Periodic validation of lifecycle invariants  | Repair incomplete promotions and expired leases |

SurrealDB supports indexes over expiration fields. For example:

```

DEFINE INDEX checkpoint_expiry
ON TABLE checkpoint
FIELDS expires_at;

-- Retrieve a bounded batch of expiration candidates.
LET $candidates = SELECT VALUE id
FROM checkpoint
WHERE expires_at < time::now()
LIMIT 250;

```

The controller would then validate that these candidates are unprotected, unreferenced by active work, and still eligible for deletion. Those checks and the relevant deletion should occur transactionally, with appropriate conflict handling.

Use a two-phase reclamation policy for shared numerical data:

`eligible → tombstoned → grace period → reference recheck → physically deleted`

This provides a recovery window for late-running agents or workers, avoids destroying state during a concurrent promotion, and lets deletion resume safely after a crash.

### Native graph references are especially useful

SurrealDB supports `REFERENCE` fields with `ON DELETE IGNORE`, `UNSET`, `CASCADE`, `REJECT`, or custom `THEN` logic.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



For example, a published result can protect its checkpoint:

```

DEFINE FIELD checkpoint ON result_manifest
TYPE record<checkpoint>
REFERENCE ON DELETE REJECT;

```

With this contract, deleting a checkpoint referenced by a published result fails rather than silently invalidating the result.

A checkpoint could similarly hold references to its constituent numerical-state chunks.

I would reserve `CASCADE` for genuinely exclusive ownership, such as deleting child diagnostic records belonging to a discarded test. For shared or deduplicated state, `REJECT` and reference-aware garbage collection are safer.

## 5. SurrealDB-native features worth exploiting

Several recent capabilities are particularly relevant to simulation lifecycles.

| Feature                       | Simulation use case                                               | Important consideration                                      |
| ----------------------------- | ----------------------------------------------------------------- | ------------------------------------------------------------ |
| `DEFINE EVENT`                | Enforce transitions, update metadata, register audit events       | Triggered by writes, not elapsed time                        |
| Async events (3.x)            | Deferred internal reconciliation, post-commit processing          | At-least-once execution; make handlers idempotent            |
| `BEGIN` / `COMMIT`            | Atomically publish checkpoints and update active run pointers     | Keep large payload writes out of long transactions           |
| `SELECT ... FOR UPDATE` (3.3) | Protect state transitions and competing cleanup operations        | Optimistic conflict detection; retry aborted transactions    |
| `CHANGEFEED`                  | Recover lifecycle notifications missed while a worker was offline | Change history expires; not a permanent simulation record    |
| `LIVE SELECT`                 | Watch running, paused, failed, and completed simulations          | Subscription mechanism, not durable historical replay        |
| `VERSION` queries             | Inspect prior record values or diagnose state mutations           | Requires a versioned backend and retained history            |
| Array-based record IDs        | Efficiently retrieve all checkpoints for a run or time window     | Design key ordering around the most frequent access patterns |
| `ALTER TABLE ... COMPACT`     | Reclaim/organize underlying storage after heavy churn             | Engine-dependent; administrative operation                   |

Sources:&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+6



### A useful distinction: checkpoints versus database history

SurrealDB 3.3 supports time-travel queries on version-enabled RocksDB and SurrealKV storage engines.

For example, a self-hosted instance can be configured using:

```

surreal start --user root --pass secret \
  "surrealkv://simulation.db?versioned=true&retention=30d"

```

A query can then inspect a record at an earlier point in time:

```

SELECT * FROM simulation_run:example
VERSION time::now() - 1h;

```

The `retention=30d` setting controls historical versions, not ordinary record expiration. The datastore's historical retention setting is also distinct from per-run checkpoint policies.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



This is useful for forensic investigation and debugging, but I would not make it the primary simulation-resumption mechanism.

A durable checkpoint should contain a validated, explicitly identified simulation state, including the numerical payload and everything needed for restart. Historical database versions are not a substitute for that contract.

## 6. The most relevant tooling integrations

There is more purpose-built tooling available than simply combining the SurrealDB SDK with custom scripts.

| Tool                     | Capabilities and application                                                     | Priority                      |
| ------------------------ | -------------------------------------------------------------------------------- | ----------------------------- |
| SurrealKit               | Official schema management, migrations, seeds, isolated database tests, rollouts | Very high                     |
| SurrealKit Testing       | Declarative suites using isolated ephemeral databases                            | Very high                     |
| Testcontainers SurrealDB | Spin up isolated instances for Rust integration tests                            | High                          |
| SurrealDB Rust SDK       | Typed operations, transactions, embedded/remote access                           | High                          |
| SurrealDB Embedded MCP   | Agent access to schemas, SurrealQL, database operations                          | High for coding agents        |
| SurrealDB Observability  | Prometheus metrics, OpenTelemetry metrics/logs/traces                            | High                          |
| SurrealDB CLI            | Query execution, export/import, scripted administration                          | High                          |
| surrealctl               | Cloud instance provisioning, logs, metrics, backups, controls                    | Conditional on Cloud          |
| Backup and recovery      | Recovery-grade exports and storage snapshots                                     | Essential for durable results |

### SurrealKit is especially interesting

SurrealKit has become the official SurrealDB schema-management tool. The previous `surrealdb-migrations` project is archived in its favor.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



It directly addresses some of your concerns:

For scratch and test environments, `surrealkit sync` provides declarative schema reconciliation and can watch schema files for changes. Its test framework creates an isolated ephemeral database for each suite, so tests need not leave persistent fixtures and temporary records behind.

For canonical persistent environments, SurrealKit Rollouts support staged schema migration with explicit expand, application cutover, and contract phases.

For consistent implementation, SurrealKit also provides Rust library integration, schema embedding, typed schema generation, seed management, and machine-readable test results.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+4



Its newer 1.0 capabilities are documented against prerelease `1.0.0-beta.1`; the stable CLI line remains 0.7 in those docs.

Given your architecture, I would exploit SurrealKit for ephemeral test isolation and schema consistency, then persist only selected test evidence—such as failure cases, regression comparisons, and reproducible inputs—into your canonical simulation database.

### Lifecycle scheduler

On Ubuntu, a `systemd` timer invoking a small Rust binary may be sufficient for periodic retention sweeps.

The important part is keeping the schedule external but the job state inside SurrealDB. A maintenance-job record can track its cursor, lease, last successful sweep, error details, and reclaimed counts.

For more continuous execution, a Tokio-based worker works equally well. Neither requires a separate durable queue or database.

### Agent-facing interface

SurrealDB 3.1 introduced a built-in MCP server, exposed by self-hosted instances at `/mcp`. It supports schema inspection, querying, and record operations.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



This is useful for agents that need to identify stalled simulations, retrieve reproducible test states, or examine checkpoint lineage.

I would expose higher-level lifecycle operations through a small domain-specific interface rather than have every coding agent independently implement the retention rules. Examples are `resume_run`, `publish_checkpoint`, `pin_artifact`, `list_orphans`, and `reclaim_expired_state`.

That centralizes the guarantees without duplicating underlying persistence logic.

## 7. Numerical state storage: an additional optimization opportunity

Given your Arrow/DataFusion-oriented simulation pipeline, I would distinguish between structured, queryable state and bulk numerical payloads.

Structured state

Use ordinary SurrealDB records for run identity, solver metadata, state descriptors, configuration, dimensions, provenance, relationships, and indexed scalar values.

Bulk numerical state

For substantial state matrices or trajectories, consider storing Arrow IPC-encoded (or otherwise compactly encoded) numerical chunks and indexing them with SurrealDB manifests.

SurrealDB supports native `bytes` values, so this does not inherently require an external storage service.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



A useful chunk descriptor could be:

```

{
  "run_id": "run-123",
  "checkpoint_step": 120,
  "payload": {
    "encoding": "arrow-ipc",
    "compression": "zstd",
    "schema_fingerprint": "...",
    "content_hash": "...",
    "chunk_ids": ["...", "..."]
  },
  "retention": {
    "policy": "recoverable",
    "protected": false,
    "expires_at": null
  }
}

```

This is a conceptual application schema, not a literal SurrealQL record.

The payoff is that large numerical payloads can be persisted once and reused by multiple checkpoints or result manifests.

There are two potentially helpful newer capabilities.

SurrealDB 3.3 streaming: gRPC results and streaming query APIs allow incremental retrieval rather than materializing every result at once. This can reduce memory pressure when reconstructing substantial simulation state.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

SurrealDB



DataFusion `TableProvider`: a custom provider can expose SurrealDB-backed state to DataFusion through Arrow `RecordBatch` streams, allowing execution to remain integrated into the existing relational compute fabric. Predicate/projection pushdown and format conversion would still have to be implemented appropriately.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://datafusion.apache.org\&sz=32)

Apache DataFusion Blog



An important tradeoff: opaque Arrow payloads will not automatically become queryable SurrealDB fields. Keep frequently filtered metadata and numerical summary statistics in normal indexed records.

### SurrealDB file buckets are another option

SurrealDB 3.x also introduced `DEFINE BUCKET` and file pointers. Buckets can be backed by memory or filesystem storage; 3.3 added S3-compatible, Google Cloud Storage, and Azure backends.

This is potentially useful for larger simulation artifacts while keeping file operations under the SurrealDB interface. However, the feature is still marked experimental. Filesystem and object-store backends also introduce additional physical storage and backup considerations, even though they are managed through SurrealDB.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



For a strict single-persistence-substrate design, I would evaluate native `bytes` records first, and file buckets separately if payload size or throughput justifies them.

## 8. Recommended overall architecture

I would target the following operating model:

| Responsibility                  | Canonical implementation                                    |
| ------------------------------- | ----------------------------------------------------------- |
| Scenario and model definitions  | Versioned SurrealDB records                                 |
| Running simulation coordination | `simulation_run`, `run_attempt`, and lease records          |
| Paused/resumable simulations    | Immutable checkpoints with explicit restart contracts       |
| Long-term results               | Result manifests referencing protected artifacts            |
| Short-lived tests               | SurrealKit isolated ephemeral databases                     |
| Failure reproduction            | Selected immutable test evidence in the persistent database |
| Retention rules                 | Versioned policy records and schema-level invariants        |
| Lifecycle transitions           | Transactions, `DEFINE EVENT`, and Rust orchestration        |
| Automatic cleanup               | Lightweight scheduled controller with reference-aware GC    |
| Historical debugging            | Changefeeds and optional versioned storage                  |
| Schema management               | SurrealKit                                                  |
| Agent interaction               | Embedded MCP plus domain-specific lifecycle tools           |
| Monitoring                      | Prometheus/OpenTelemetry and lifecycle metrics              |

There is one further design consideration I would emphasize: avoid associating the retention lifetime of one data object with the retention lifetime of the simulation that originally produced it.

A particular checkpoint may support a paused run, a subsequently published result, and a regression test baseline simultaneously. Retention should be derived from these current obligations, not simply the age or status of the originating run.

This leads to a clean separation of concerns:

- Execution state determines what can run, pause, resume, or retry.
- Artifact references determine what must continue to exist.
- Retention policy determines when unprotected artifacts become eligible for reclamation.

That separation gives you much of the operational convenience normally associated with distinct scratch databases, checkpoint stores, and permanent result repositories—without introducing additional canonical persistence systems.

The two capabilities I would prioritize beyond what SurrealDB already provides are SurrealKit for disposable test environments and a reference-aware lifecycle/garbage-collection controller. Together they address most of the fundamental lifecycle problem, while preserving a unified and auditable data model.&#x20;