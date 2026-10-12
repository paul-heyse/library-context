# SurrealDB tooling for a unified code intelligence platform

## 1. Overview and recommended tooling stack

For a code intelligence platform using SurrealDB as its single canonical persistence system—across source extraction, CPG construction, incremental analysis, temporary working state, test artifacts, and permanent results—I would focus primarily on leveraging SurrealDB's native capabilities and ecosystem tooling before building separate infrastructure.

The biggest opportunities are not necessarily in graph modeling itself. They are in lifecycle automation, isolated testing, schema management, incremental processing, search, observability, and agent access.

### Tooling priorities

| Tool or capability                       | Primary value                                                                       | Priority       |
| ---------------------------------------- | ----------------------------------------------------------------------------------- | -------------- |
| SurrealKit                               | Schema management, migrations, testing, disposable databases, schema modules        | Essential      |
| SurrealDB native lifecycle primitives    | Transactions, events, references, retention rules, database-level disposal          | Essential      |
| SurrealDB Rust SDK                       | Embedded or remote access, ingestion, transactional updates, lifecycle coordination | Essential      |
| SurrealQL indexes and query optimization | Graph traversal, lexical search, selective retrieval, query performance             | Essential      |
| Changefeeds and live queries             | Incremental processing, reactive updates, analysis notifications                    | High           |
| Embedded MCP server                      | Agent-accessible querying, introspection, and database operations                   | High           |
| SurrealDB CLI                            | Bulk import/export, database inspection, administrative automation                  | High           |
| SurrealDB observability                  | OpenTelemetry, Prometheus, diagnostic logging, performance monitoring               | High           |
| SurrealDB Studio                         | Interactive exploration, debugging, graph and query inspection                      | High           |
| Surrealism / custom database functions   | Encapsulated operations and reusable database-side logic                            | Evaluate       |
| Database versioning and change history   | Historical inspection, audit, debugging, incremental recovery                       | Evaluate       |
| Testcontainers / embedded databases      | Isolated integration tests and realistic disposable environments                    | High           |
| `surrealctl`                             | Cloud infrastructure, backups, metrics, and instance administration                 | Cloud-specific |

The architecture I would favor is one canonical SurrealDB platform, supported by standardized lifecycle controls and purpose-built ephemeral workspaces, rather than introducing separate persistence systems for temporary versus durable data.

An important qualification: SurrealKit's newest documented 1.0 capabilities currently target `1.0.0-beta.1`; installing the latest stable release still selects the 0.7 line. The distinction matters for schema modules, targets, and other newer features.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



## 2. SurrealKit — the highest-priority complementary tool

SurrealKit is the official SurrealDB schema-management and migration tool. Of the external tools, it addresses the greatest number of your lifecycle concerns directly.

| Capability               | Functionality                                                              | Application to code intelligence                                                                 |
| ------------------------ | -------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------ |
| Schema Sync              | Declarative `.surql` definitions, tracked hashes, automatic reconciliation | Keep ingestion, graph, analysis, cache, and test schemas consistent                              |
| Watch mode               | Automatically apply schema changes during development                      | Rapidly iterate on CPG schema and query infrastructure                                           |
| Schema modules           | Independently managed schema definitions with dependencies                 | Separate extraction, graph, analysis, testing, and retention concerns without separate platforms |
| Named database targets   | Apply schemas across selected databases or instances                       | Standardize persistent and disposable environments                                               |
| Rollouts                 | Phased expand/contract migrations with rollback support                    | Evolve canonical graph schemas without destructive recreation                                    |
| Declarative testing      | SQL, permissions, schema, relationships, computed behavior, HTTP APIs      | Test database contracts independently of application logic                                       |
| Ephemeral test databases | Isolated database per test suite, automatically disposed                   | Avoid contaminating the canonical code intelligence database                                     |
| `--keep-db`              | Preserve a failed test's temporary database for inspection                 | Debug graph consistency or analysis regressions                                                  |
| Seed management          | Tracked, repeatable fixture loading                                        | Initialize known code examples and reference graph structures                                    |
| Schema type generation   | Generate JSON/TypeScript definitions from the database                     | Improve schema discoverability and integration                                                   |
| Rust library integration | Invoke schema functionality programmatically                               | Integrate migrations and validation into a Rust-based harness                                    |
| CI integration           | Machine-readable reports, GitHub Actions support                           | Reproducible automated database validation                                                       |

The particularly useful combination is SurrealKit schema modules + isolated test databases + structured test output.

For example, you can maintain distinct logical schema modules for reusable code evidence, graph structures, analysis records, and temporary execution state while applying those definitions to different databases through one configuration.

SurrealKit 1.0 supports dependency-ordered modules and independently tracked schema ownership. However, its current beta only fully applies the module/target selection matrix to `sync`; other commands, including `test` and `seed`, still have targeting limitations.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



### Testing features worth emphasizing

SurrealKit currently provides five distinct test categories:

| Test category        | What it verifies                                     |
| -------------------- | ---------------------------------------------------- |
| `sql_expect`         | SurrealQL success/failure and returned values        |
| `permissions_matrix` | Create/read/update/delete access behavior            |
| `schema_metadata`    | Field, index, and definition contracts               |
| `schema_behavior`    | Computed fields, functions, and record relationships |
| `api_request`        | SurrealDB-hosted HTTP API contracts                  |

These are useful for validating the database layer independently of your compiler extractors and graph-analysis algorithms.

SurrealKit can execute suites in parallel, filter by test case or tag, retain test databases for debugging, and produce JSON reports.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



One important limitation: SurrealKit's tests are primarily database-contract tests. They complement rather than replace tests that verify whether a compiler extractor has generated the correct CPG.

## 3. Native SurrealDB lifecycle-management capabilities

This is where I would concentrate the most attention for managing temporary and long-term persistence without adding another database or queue.

| Capability                      | Native mechanism                           | Value for code intelligence                                          |
| ------------------------------- | ------------------------------------------ | -------------------------------------------------------------------- |
| Disposable workspaces           | `REMOVE DATABASE`, `REMOVE TABLE`          | Delete entire test or temporary analysis environments                |
| Transactional state transitions | `BEGIN`, `COMMIT`, `CANCEL`, `FOR UPDATE`  | Publish validated analyses without exposing incomplete results       |
| Synchronous triggers            | `DEFINE EVENT`                             | Enforce lifecycle rules and maintain dependent records               |
| Asynchronous triggers           | `DEFINE EVENT ... ASYNC`                   | Defer follow-on database work without an additional job system       |
| Reusable database operations    | `DEFINE FUNCTION`                          | Centralize lifecycle operations and avoid repeated client-side logic |
| Referential protection          | `REFERENCE ON DELETE REJECT`               | Prevent deletion of artifacts needed by retained results             |
| Reference-based cleanup         | Reverse record references, graph traversal | Find orphans and identify objects still in use                       |
| Expiration eligibility          | Indexed datetime fields                    | Identify expired cache, staging, and test records                    |
| Historical change replay        | `CHANGEFEED`, `SHOW CHANGES`               | Recover from interrupted ingestion or analysis processing            |
| Live notifications              | `LIVE SELECT`                              | Update agents or user interfaces when analyses complete              |
| Time-travel queries             | `SELECT ... VERSION`                       | Examine historical record states when versioning is enabled          |
| Storage reclamation             | `ALTER ... COMPACT`                        | Reorganize persistent storage following heavy churn                  |

### Particularly valuable capabilities

A. Disposable databases and asynchronous reclamation

Beginning with SurrealDB 3.2, removing a database immediately removes it from the catalog while physical key deletion proceeds through a background reaper.

This is valuable for high-frequency tests and temporary full-codebase indexing experiments. A large disposable test database can be discarded as a unit rather than requiring application-level traversal and deletion of every associated node, edge, diagnostic, and fixture.

B. Asynchronous events

Since 3.0, SurrealDB supports asynchronous events through `DEFINE EVENT ... ASYNC`, including retry configuration.

These are database-managed and enqueued transactionally, with at-least-once execution. They are useful for registering analysis follow-up work, updating dependent status records, or maintaining auxiliary state after ingestion.

The important limitation is that an event is triggered by a database change. It is not a general time-based scheduler.

C. Reference-aware deletion

`REFERENCE ON DELETE REJECT` can directly enforce that permanent artifacts are not deleted while other records reference them. `CASCADE` and `UNSET` support alternative ownership and cleanup semantics.

For a platform containing permanent graph evidence, temporary derived analyses, and shared extraction results, these native integrity guarantees can displace a meaningful amount of custom cleanup validation.

D. Automatic expiration

SurrealDB's general database engine still does not provide automatic per-record TTL deletion. Its documentation describes indexed expiration fields, mutation-triggered cleanup, or externally scheduled deletion.

A `systemd` timer or Tokio maintenance worker, using SurrealDB for its own job state, is a lightweight solution. Because expiry eligibility and deletion safety are distinct concerns, the worker should verify references before deleting shared evidence.

E. Storage compaction

`ALTER TABLE`, `ALTER DATABASE`, and `ALTER SYSTEM` support compaction on compatible storage engines. This is useful after extensive test and analysis churn, though it is a maintenance operation rather than a substitute for record deletion.

Sources:&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+5



## 4. Native graph, indexing, and search tooling

For code intelligence, these deserve more attention than a separate graph-processing service or auxiliary search database.

| Capability                    | Native tooling                                    | Relevance                                                         |
| ----------------------------- | ------------------------------------------------- | ----------------------------------------------------------------- |
| Graph relationships           | `RELATE`, relation tables, graph traversals       | Canonical CPG edges and semantic relationships                    |
| Lightweight graph edges (3.3) | `TYPE RELATION ... LIGHTWEIGHT`                   | More compact storage for property-free relationships              |
| Cached adjacency (3.3)        | `INLINE EDGES`, `INLINE REFERENCES`               | Reduce reads for frequently traversed graph neighborhoods         |
| Bidirectional references      | `REFERENCE` fields and reverse traversal          | Navigate provenance, dependencies, and artifact ownership         |
| Composite and unique indexes  | `DEFINE INDEX ... FIELDS`, `UNIQUE`               | Fast identity, lookup, and deduplication                          |
| Count indexes                 | `COUNT`, `COUNT WHERE`                            | Quickly retrieve selected graph or analysis cardinalities         |
| Native lexical retrieval      | `FULLTEXT ANALYZER ... BM25`                      | Search names, definitions, documentation, and indexed source text |
| Custom tokenization           | `DEFINE ANALYZER`                                 | Code-oriented identifier splitting, stemming, prefixes, n-grams   |
| Vector retrieval              | `HNSW`, `DISKANN`                                 | Optional semantic retrieval without a second vector database      |
| Hybrid ranking                | `search::rrf`, `search::linear`                   | Fuse lexical and vector result sets within SurrealQL              |
| Materialized views            | `DEFINE TABLE ... AS SELECT`                      | Maintain reusable relational projections and summaries            |
| Query plan inspection         | `EXPLAIN`, `EXPLAIN ANALYZE FORMAT JSON`          | Identify expensive queries and inefficient execution plans        |
| Index diagnostics             | `INFO FOR INDEX`, `REBUILD INDEX`, `CONCURRENTLY` | Track builds, repair indexes, and manage index evolution          |

### Three capabilities I would investigate first

Lightweight edges: SurrealDB 3.3 introduced relation tables without independent edge records. They are useful for relationships whose existence is the entire fact, and whose edges need no additional fields. For richly attributed or individually versioned edges, normal relation records remain necessary.

Inline adjacency caches: `INLINE EDGES` and `INLINE REFERENCES` cache small adjacency sets directly with records. This can be particularly valuable for agents that request many small graph neighborhoods rather than executing one enormous traversal. The tradeoff is additional write maintenance and bounded cache effectiveness for very high-degree nodes.

Code-aware full-text analyzers: SurrealDB provides a built-in `camel` tokenizer as well as class/punctuation tokenizers, n-grams, edge n-grams, and configurable transformations. These permit domain-specific lexical search without maintaining a separate search engine.

There is also a notable improvement to query diagnostics in 3.x: standalone `EXPLAIN ANALYZE FORMAT JSON` provides structured operator and execution metrics. This is valuable for systematic query benchmarking, although the planner's diagnostic JSON shape is explicitly not guaranteed to remain stable.

Sources:&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+5



## 5. Rust SDK and embedded database tooling

The Rust SDK is particularly important because it provides consistent access to SurrealDB whether the database is embedded in a process or accessed remotely.

| Capability                   | Tooling                                         | Value                                                                |
| ---------------------------- | ----------------------------------------------- | -------------------------------------------------------------------- |
| Embedded in-memory database  | `kv-mem`                                        | Fast, isolated tests without operating a database server             |
| Embedded persistent database | `kv-rocksdb`, `kv-surrealkv`                    | Local persistence using the same SurrealDB programming model         |
| Remote database              | `protocol-ws`, `protocol-http`, `protocol-grpc` | Shared canonical persistence accessible to multiple processes        |
| Typed records                | `SurrealValue` derive                           | Structured Rust serialization and deserialization                    |
| Batch operations             | SDK insert/query methods                        | Efficient ingestion of extracted records and graph edges             |
| Client-side transactions     | Transaction APIs                                | Coordinate multi-operation writes                                    |
| Live subscriptions           | SDK live query support                          | React to analysis completion and state transitions                   |
| Incremental result streaming | `.stream_items()` with gRPC                     | Process large query result sets without buffering the whole response |

### Particularly relevant: new gRPC streaming

SurrealDB 3.3 introduced a gRPC transport for the Rust SDK.

Unlike WebSocket and HTTP queries, which normally buffer query responses in the Rust SDK, gRPC supports actual incremental result streaming through `.stream_items()`.

For a code intelligence system that may retrieve thousands of nodes and graph relationships, this can significantly improve memory behavior and time to first result.

The SDK supports `grpc://` and `grpcs://` via the `protocol-grpc` feature.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



### Testcontainers versus embedded databases

Both are worth retaining as complementary testing approaches.

| Tool                                | Best use                                                                                                             |
| ----------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Embedded SurrealDB `kv-mem`         | Lightweight unit and integration tests requiring real SurrealQL semantics                                            |
| SurrealKit ephemeral test databases | Declarative schema, permissions, query, and database-contract tests                                                  |
| Testcontainers SurrealDB module     | Integration testing involving real server processes, network behavior, authentication, and deployment configurations |

Testcontainers has a community-maintained Rust SurrealDB module exposed through `testcontainers-modules` with the `surrealdb` feature.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://testcontainers.com\&sz=32)

Testcontainers



The value is being able to use the same database technology at every testing level without requiring every test to interact with the long-lived canonical datastore.

## 6. Agent interfaces and database-side extensions

Given the heavy reliance on coding agents, SurrealDB now has several directly relevant integration capabilities.

| Tool                            | Capability                                                                            | Recommendation                     |
| ------------------------------- | ------------------------------------------------------------------------------------- | ---------------------------------- |
| Built-in MCP server             | Database inspection, SurrealQL queries, CRUD, relation operations, function execution | High                               |
| Official SurrealDB Agent Skills | Up-to-date SurrealQL, vector-search, and Python SDK guidance                          | High                               |
| `DEFINE FUNCTION`               | Reusable transactional database functions                                             | High                               |
| `DEFINE API`                    | Database-hosted HTTP endpoints                                                        | Conditional                        |
| Surrealism                      | Rust/WASM extensions executed within SurrealDB                                        | Experimental; evaluate selectively |
| ISO GQL (Cypher-style)          | Standard graph-pattern querying against existing SurrealDB relations                  | High for graph-intensive workflows |
| GraphQL API                     | Typed, declarative access for client applications                                     | Optional                           |
| PostgreSQL wire protocol        | Connect existing PostgreSQL clients and drivers                                       | Optional interoperability layer    |

### Built-in MCP

SurrealDB 3.1 introduced a first-party MCP server with tools for querying, schema inspection, record manipulation, and function invocation.

Two deployment modes matter:

`surreal mcp` starts an embedded database and communicates over stdio. It grants owner-level tool access and is best suited to a trusted local development environment.

`surreal start` with `/mcp` exposes MCP on an existing SurrealDB instance, with the database's normal authentication and access controls. This is more appropriate when coding agents should operate against the same persistent database used by the application.

The built-in interface already covers many generic operations. A domain-specific agent tool layer should therefore focus on higher-level operations that the built-in MCP does not know how to perform safely, rather than duplicating generic CRUD and query tools.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database



SurrealDB also publishes official agent skills:

```
npx skills add surrealdb/agent-skills
```

These are especially useful for reducing reliance on potentially outdated model knowledge about SurrealQL syntax and capabilities.

### Database-side functions and extensions

`DEFINE FUNCTION` provides reusable SurrealQL operations, including operations that read and write records transactionally.

`DEFINE API` exposes custom HTTP endpoints implemented within the database.

Surrealism permits Rust functions compiled to WebAssembly to execute within SurrealDB, supported by the `surreal module` CLI for initialization, building, and inspection.

Surrealism is still experimental and requires explicit capabilities to be enabled. I would prioritize SurrealQL functions first, considering WASM extensions only where functionality or execution locality justifies the additional complexity.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+3



### ISO GQL and PostgreSQL interoperability

SurrealDB 3.3 also makes ISO GQL available by default. This provides graph-pattern operations such as `MATCH ... RETURN` directly against existing SurrealDB nodes and edges, without a parallel graph database.

Its PostgreSQL wire protocol is useful for existing drivers and tooling, but it does not mean SurrealDB supports general PostgreSQL/ANSI SQL syntax. Clients currently send SurrealQL or ISO GQL.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

Database

+1



## 7. Operational tooling, query diagnostics, and observability

| Tool                        | Capability                                                 | Application                                            |
| --------------------------- | ---------------------------------------------------------- | ------------------------------------------------------ |
| `surreal sql`               | Interactive querying and scripts                           | Inspection, maintenance, controlled cleanup            |
| `surreal validate`          | SurrealQL syntax validation                                | Automatically check generated queries before execution |
| `surreal format`            | Built-in SurrealQL formatting (3.3+)                       | Normalize individual query and schema files            |
| `surqlfmt`                  | File/glob formatting, `--check`, style controls            | CI formatting and automated query normalization        |
| `surreal export` / `import` | Logical database export and restoration                    | Backups, reproducible test data, recovery              |
| SurrealDB Studio            | Visual query editor, table/graph explorer, schema designer | Interactive inspection and debugging                   |
| `INFO FOR INDEX`            | Index state, build progress, compaction information        | Diagnose indexing performance                          |
| `EXPLAIN ANALYZE`           | Detailed query execution diagnostics                       | Graph query performance optimization                   |
| OpenTelemetry               | Distributed tracing and metrics                            | Connect ingestion, analysis, and database operations   |
| Prometheus `/metrics`       | Database-level performance metrics                         | Observe throughput, latency, resource pressure         |
| `surrealctl`                | Cloud instance, credentials, metrics, logs, backups        | Managed Cloud environments                             |

There are three capabilities I would explicitly incorporate into automated development workflows.

Query validation and formatting. The `surreal validate` command can validate `.surql` files without executing them. `surqlfmt --check` can enforce consistent formatting. These are low-overhead tools for coding agents generating large amounts of schema and query code.

Query execution analysis. `EXPLAIN ANALYZE FORMAT JSON` and `INFO FOR INDEX` provide visibility into index usage, operator behavior, and execution performance. They are especially helpful for comparing graph query alternatives using measurements rather than assumptions.

Integrated observability. SurrealDB 3.1 unified its metrics and tracing around OpenTelemetry. Community Edition supports Prometheus and OTLP; Enterprise adds capabilities such as durable audit and detailed slow-query pipelines. A separate Grafana/Tempo/Jaeger stack is optional operational infrastructure, not a separate canonical code-intelligence datastore.

SurrealDB Studio further provides tabular and graph result visualization, schema exploration, a query history, and interactive record inspection.&#x20;

[image](https://www.google.com/s2/favicons?domain=https://surrealdb.com\&sz=32)

SurrealDB

+5



## 8. Overall recommendation

I would focus on the following capability bundles, in roughly this order.

| Bundle                                     | Tooling to combine                                                                    | Outcome                                                               |
| ------------------------------------------ | ------------------------------------------------------------------------------------- | --------------------------------------------------------------------- |
| A. Standardized database management        | SurrealKit Sync, Rollouts, schema modules, seeds                                      | Consistent, version-controlled SurrealDB environments                 |
| B. Disposable execution and testing        | SurrealKit Testing, embedded memory databases, Testcontainers, `REMOVE DATABASE`      | Isolated test and scratch lifecycles with inexpensive disposal        |
| C. Automatic retention and integrity       | `DEFINE EVENT`, `DEFINE FUNCTION`, `REFERENCE`, expiration indexes, scheduled cleanup | Centralized lifecycle enforcement without another persistence service |
| D. Incremental and reactive processing     | Changefeeds, async events, live queries, Rust SDK                                     | Resumable indexing, deferred analysis, reactive agent notifications   |
| E. Native retrieval and graph acceleration | Lightweight relations, inline adjacency, BM25, custom analyzers, indexes              | Better graph/search performance while keeping one query platform      |
| F. Agent-facing database operations        | Embedded MCP, Agent Skills, ISO GQL                                                   | Richer agent access with less bespoke infrastructure                  |
| G. Diagnostics and operational control     | `EXPLAIN ANALYZE`, `INFO`, Studio, Prometheus, OTLP, CLI                              | Measurable, inspectable, maintainable database operations             |

The most valuable relatively recent features to investigate are probably SurrealKit 1.0's schema modules and test environments, SurrealDB 3.3's lightweight relations and inline adjacency caches, native ISO GQL, and gRPC result streaming.

Together with the native lifecycle features, these provide a substantial foundation for managing scratch state, durable analysis results, and automated testing while preserving SurrealDB as the sole canonical persistence platform.

## 9. Web sources — complete Markdown-copyable links

All references below use explicit URLs rather than application-specific link objects.

### SurrealKit and database testing

1. SurrealKit overview, installation, and capabilities — https\://surrealdb.com/docs/manage/schema-migration
2. SurrealKit Schema Sync and watch mode — https\://surrealdb.com/docs/manage/schema-migration/sync
3. Schema modules and database targets — https\://surrealdb.com/docs/manage/schema-migration/modules-and-targets
4. SurrealKit Rollouts and migrations — https\://surrealdb.com/docs/manage/schema-migration/rollouts
5. SurrealKit Testing framework — https\://surrealdb.com/docs/manage/schema-migration/testing
6. SurrealKit CI/CD integration — https\://surrealdb.com/docs/manage/schema-migration/testing/ci-cd
7. Testcontainers SurrealDB module — https\://testcontainers.com/modules/surrealdb/

### Native lifecycle and state management

8. Transactions and conflict handling — https\://surrealdb.com/docs/learn/querying/concepts-and-guides/transactions
9. DEFINE EVENT and asynchronous events — https\://surrealdb.com/docs/reference/query-language/statements/define/event
10. Reactive lifecycle patterns and expiration — https\://surrealdb.com/docs/learn/schema-management/events-and-triggers/reactive-patterns
11. Record references and deletion policies — https\://surrealdb.com/docs/reference/query-language/language-primitives/record-references
12. DEFINE FUNCTION and stored procedures — https\://surrealdb.com/docs/reference/query-language/statements/define/function
13. REMOVE database, tables, and other resources — https\://surrealdb.com/docs/reference/query-language/statements/remove
14. Changefeeds and change replay — https\://surrealdb.com/docs/learn/querying/real-time/changefeeds
15. Live queries and subscriptions — https\://surrealdb.com/docs/learn/querying/real-time/live-queries
16. Time-travel and versioned queries — https\://surrealdb.com/docs/reference/query-language/statements/select
17. Storage compaction — https\://surrealdb.com/docs/reference/query-language/statements/alter/table

### Graph, indexing, and query optimization

18. DEFINE TABLE: relations, lightweight edges, inline adjacency and views — https\://surrealdb.com/docs/reference/query-language/statements/define/table
19. DEFINE INDEX: composite, full-text, count, HNSW and DiskANN indexes — https\://surrealdb.com/docs/reference/query-language/statements/define/indexes
20. DEFINE ANALYZER: tokenizers and filters — https\://surrealdb.com/docs/reference/query-language/statements/define/analyzer
21. Search functions, hybrid ranking and RRF — https\://surrealdb.com/docs/reference/query-language/functions/database-functions/search
22. EXPLAIN and EXPLAIN ANALYZE — https\://surrealdb.com/docs/reference/query-language/statements/explain
23. ISO GQL graph query language — https\://surrealdb.com/docs/learn/querying/gql/overview

### Rust and agent integrations

24. SurrealDB Rust SDK — https\://surrealdb.com/docs/reference/rust
25. Rust embedded database engines — https\://surrealdb.com/docs/reference/rust/embedding
26. Rust SDK connection modes, including gRPC — https\://surrealdb.com/docs/reference/rust/methods/connect
27. Embedded SurrealDB MCP server — https\://surrealdb.com/docs/build/ai-agents/mcp/embedded
28. Official SurrealDB Agent Skills — https\://surrealdb.com/docs/build/ai-agents/agent-skills
29. DEFINE API for database-hosted endpoints — https\://surrealdb.com/docs/reference/query-language/statements/define/api
30. DEFINE MODULE / Surrealism — https\://surrealdb.com/docs/reference/query-language/statements/define/module
31. Surrealism module CLI — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/module
32. PostgreSQL wire protocol — https\://surrealdb.com/docs/reference/rest-api/postgres-protocol

### Operations, administration, and diagnostics

33. SurrealDB CLI overview — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/overview
34. SurrealQL validation CLI — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/validate
35. Built-in SurrealQL formatter — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/format
36. SurrealQL formatter (`surqlfmt`) — https\://surrealdb.com/docs/reference/cli/formatter/overview
37. SurrealDB Studio — https\://surrealdb.com/docs/explore/studio
38. Observability: metrics, traces and logs — https\://surrealdb.com/docs/manage/observability
39. SurrealDB self-hosted backup and recovery — https\://surrealdb.com/docs/manage/self-hosted/backups-and-recovery
40. SurrealDB CLI export — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/export
41. SurrealDB CLI import — https\://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/import
42. `surrealctl` Cloud management — https\://surrealdb.com/docs/reference/cli/surrealctl/overview

Sources reviewed October 12, 2026. Version-specific and prerelease capabilities should be checked against the installed SurrealDB and SurrealKit versions before implementation.