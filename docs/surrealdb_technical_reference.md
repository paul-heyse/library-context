SurrealDB: Comprehensive Technical Reference & Tooling Directory
Updated October 15, 2026 | Target audience: Advanced LLM programming agents
SurrealDB's documentation now covers considerably more than its core database and SurrealQL. Its ecosystem includes embedded storage engines, graph and vector indexing, schema migration tooling, Rust and Python SDKs, observability, data integration, WASM extensions, first-party MCP servers, and AI-agent memory infrastructure.
The resources below are organized by functional domain, with an explanation of what each contributes. All URLs are provided in full, as plain text inside code formatting, so they can be copied directly into Markdown or agent instructions.
1. Foundational resources and agent-readable documentation
These are the highest-priority entry points. SurrealDB provides an especially useful capability for programming agents: its entire documentation corpus is available in Markdown, independently of Context7.
Resource / Topic	Full URL
Official documentation — Entry point covering database concepts, implementation, administration, integrations, and API references.	https://surrealdb.com/docs
Complete documentation index — Machine-readable index of documentation pages, including their descriptions and URLs.	https://surrealdb.com/docs/llms.txt
Complete documentation corpus — Full documentation aggregated into one Markdown-oriented resource, suitable for agent retrieval or indexing.	https://surrealdb.com/docs/llms-full.txt
SurrealDB overview — Architecture, supported data models, indexing, transactions, security, and deployment models.	https://surrealdb.com/docs/what-is-surrealdb
SurrealQL reference — Canonical language documentation, including statements, functions, data types, and operators.	https://surrealdb.com/docs/reference/query-language
Release notes — All current version lines, patch-level changes, breaking changes, bug fixes, and improvements.	https://surrealdb.com/releases
SurrealDB 3.3 release notes — Current stable release line, including query execution, PostgreSQL connectivity, graph performance, and indexing improvements.	https://surrealdb.com/releases/3.3
Important agent-specific functionality: Individual documentation pages can also be requested as Markdown by appending .md to their URL or by sending Accept: text/markdown. This allows agents to retrieve focused, up-to-date technical documentation without processing HTML.
SurrealDB 3.3 is the latest stable release line as of October 15, 2026. The directory below principally reflects the 3.x capabilities; agents working with older versions should consult the appropriate migration documentation before adopting newer syntax. 

Database
+3



2. Architecture, data models, and schema design
These resources establish the database's conceptual architecture and the mechanisms for representing related information without maintaining separate document, relational, graph, and vector systems.
Resource / Topic	Full URL
Database architecture — Query execution layer, storage abstraction, transaction processing, and separation of compute and persistence.	https://surrealdb.com/docs/learn/data-models/architecture
Data models overview — How relational, document, graph, vector, geospatial, and time-series representations coexist.	https://surrealdb.com/docs/learn/data-models
Document model — Nested documents, objects, arrays, and document-oriented storage.	https://surrealdb.com/docs/learn/data-models/document/overview
Graph model — Nodes, typed relationships, edge records, and graph-native representations.	https://surrealdb.com/docs/learn/data-models/graph/overview
Creating graph relations — Relationship definitions and creation of graph edges with SurrealQL.	https://surrealdb.com/docs/learn/data-models/graph/creating-relations
Graph traversal — Traversal operators and querying connected records.	https://surrealdb.com/docs/learn/data-models/graph/graph-traversal
Recursive traversals — Multi-hop and recursive graph querying, including traversal constraints.	https://surrealdb.com/docs/learn/data-models/graph/recursive-traversals
Record links vs. graph relations — Architectural choice between direct record references and graph edge records.	https://surrealdb.com/docs/learn/data-models/graph/record-links-vs-graph-relations
Knowledge graph patterns — Techniques for modeling knowledge graphs and interconnected entities.	https://surrealdb.com/docs/learn/data-models/graph/knowledge-graph-patterns
Schema management — Overview of tables, fields, events, indexes, and schema definition mechanisms.	https://surrealdb.com/docs/learn/schema-management
Schema design — Schemafull vs. schemaless modeling and associated trade-offs.	https://surrealdb.com/docs/learn/schema-management/schema-design
Tables — Table definitions, types, views, and table-level behavior.	https://surrealdb.com/docs/learn/schema-management/tables-and-fields/tables
Fields and validation — Field types, constraints, assertions, and validation within the database.	https://surrealdb.com/docs/learn/schema-management/tables-and-fields/fields-and-validation
Record IDs and addressing — Record identity, ID formats, and addressable records.	https://surrealdb.com/docs/learn/schema-management/tables-and-fields/record-ids-and-addressing
Namespace and database architecture — Logical hierarchy, multi-tenancy, and environmental isolation.	https://surrealdb.com/docs/learn/schema-management/multi-tenancy/namespace-and-database-architecture
Schema evolution — Modifying existing schema definitions as applications evolve.	https://surrealdb.com/docs/learn/schema-management/schema-design/schema-evolution
Computed fields — Derived attributes defined and evaluated by the database rather than duplicated in application logic.	https://surrealdb.com/docs/learn/schema-management/computed-data/computed-fields
Events and triggers — Database-side reactions to data changes and their execution semantics.	https://surrealdb.com/docs/learn/schema-management/events-and-triggers/defining-events
Particularly important for code intelligence: Record links, typed relations, recursive traversal, computed fields, and schema-enforced constraints are central to representing functions, symbols, dependencies, evidence, and execution results as one interconnected model. The distinction between record references and explicit relations is especially consequential for storage and query performance. 

Database
+2



3. SurrealQL, query execution, and transactions
This section covers the expressive capabilities of SurrealQL, together with mechanisms for query correctness, reusable operations, planning, and execution efficiency.
Resource / Topic	Full URL
SurrealQL conceptual guide — Overview of the language and its differences from conventional SQL.	https://surrealdb.com/docs/learn/querying/surrealql/what-is-surrealql
Statement reference — Complete syntax and semantics of supported SurrealQL statements.	https://surrealdb.com/docs/reference/query-language/statements/overview
Built-in database functions — Functions for arrays, objects, mathematics, strings, temporal values, geometry, vectors, and other operations.	https://surrealdb.com/docs/reference/query-language/functions/database-functions
Data types — Complete SurrealQL type system and value representations.	https://surrealdb.com/docs/reference/query-language/language-primitives/data-types
Operators — Comparison, logical, graph, containment, and other operators.	https://surrealdb.com/docs/reference/query-language/language-primitives/operators
Record references — Reference semantics and ways to model relationships between records.	https://surrealdb.com/docs/reference/query-language/language-primitives/record-references
SELECT — Query syntax, filtering, projection, aggregation, and result construction.	https://surrealdb.com/docs/reference/query-language/statements/select
DEFINE — Database-native definitions for schemas, indexes, APIs, functions, access methods, and other resources.	https://surrealdb.com/docs/reference/query-language/statements/define/overview
RELATE — Creating and modifying graph relationships.	https://surrealdb.com/docs/reference/query-language/statements/relate
UPSERT — Record creation and modification with upsert semantics.	https://surrealdb.com/docs/reference/query-language/statements/upsert
Parameterised queries — Passing values safely without constructing SurrealQL strings dynamically.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/parameterised-queries
Subqueries and advanced patterns — Composable query structures and more advanced SurrealQL expressions.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/subqueries-and-advanced-patterns
Custom functions — Reusable database-side operations defined in SurrealQL.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/custom-functions
Query optimisation — Query planning, index selection, and identifying execution inefficiencies.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/query-optimisation
EXPLAIN — Execution plan inspection and associated planning details.	https://surrealdb.com/docs/reference/query-language/clauses/explain
Performance best practices — Query costs, performance analysis, and optimization considerations.	https://surrealdb.com/docs/learn/querying/performance/performance-best-practices
Transactions — ACID guarantees, snapshot isolation, conflict detection, and transaction boundaries.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/transactions
Idempotent operations — Retry-safe mutation patterns and concurrency considerations.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/idempotent-operations
Error handling — Query failures, transaction errors, and error propagation.	https://surrealdb.com/docs/learn/querying/concepts-and-guides/error-handling
The transaction documentation is especially important for agents implementing concurrent workers or simulation execution. SurrealDB uses snapshot isolation with write-conflict detection; version 3.3 also supports SELECT ... FOR UPDATE for optimistic conflict checking of specifically identified read records. This is not equivalent to PostgreSQL's blocking row locks or full serializable isolation. 

Database
+2



4. Indexing, graph retrieval, full-text search, and vector search
These resources should be examined together when designing retrieval over connected data, particularly hybrid graph-plus-text or graph-plus-vector workloads.
Resource / Topic	Full URL
Index types and strategies — Index families, indexing trade-offs, and index design.	https://surrealdb.com/docs/learn/schema-management/indexes/index-types-and-strategies
DEFINE INDEX reference — Exact index declaration syntax and available index configurations.	https://surrealdb.com/docs/reference/query-language/statements/define/indexes
Full-text search model — Native text search architecture and querying.	https://surrealdb.com/docs/learn/data-models/full-text-search/overview
Analyzers and tokenizers — Text normalization, tokenization, language handling, and indexing pipelines.	https://surrealdb.com/docs/learn/data-models/full-text-search/analyzers-and-tokenizers
Search indexes — Full-text index construction and use.	https://surrealdb.com/docs/learn/data-models/full-text-search/search-indexes
Scoring and ranking — Relevance evaluation and BM25-based retrieval.	https://surrealdb.com/docs/learn/data-models/full-text-search/scoring-and-ranking
Vector search model — Vector representation, distance functions, and semantic retrieval.	https://surrealdb.com/docs/learn/data-models/vector-search/overview
Vector indexes — HNSW and DiskANN approximate-nearest-neighbor indexing, configuration, and trade-offs.	https://surrealdb.com/docs/learn/data-models/vector-search/vector-indexes
Similarity search — Vector similarity operators and nearest-neighbor queries.	https://surrealdb.com/docs/learn/data-models/vector-search/similarity-search
Hybrid search — Combining lexical and vector relevance signals.	https://surrealdb.com/docs/learn/data-models/vector-search/hybrid-search
RAG architecture patterns — Retrieval-augmented generation using SurrealDB's combined data models.	https://surrealdb.com/docs/learn/data-models/vector-search/rag-architecture-patterns
Embedding pipelines — Creating and incorporating vector embeddings into indexed data.	https://surrealdb.com/docs/learn/data-models/vector-search/embedding-pipelines
SurrealDB 3.3 makes this area particularly relevant: indexed predicates can be combined using bitmap operations, including pre-filtering vector searches, while graph adjacency and traversal performance have also been enhanced. These capabilities should be evaluated together rather than treating graph, structured filtering, and semantic search as independent subsystems. 

Database
+2



5. Real-time capabilities, data lifecycle, and security
This group covers reactive systems, change tracking, temporal information, file storage, authorization, and data isolation. It is particularly relevant when using SurrealDB for both ephemeral processing state and durable analytical results.
Resource / Topic	Full URL
Live queries — Subscribe to record changes without repeatedly polling the database.	https://surrealdb.com/docs/learn/querying/real-time/live-queries
Changefeeds — Record-level change history and downstream change-consumption patterns.	https://surrealdb.com/docs/learn/querying/real-time/changefeeds
Real-time best practices — Subscription management, resource consumption, and operational considerations.	https://surrealdb.com/docs/learn/querying/real-time/real-time-best-practices
Reactive database patterns — Events, triggers, derived data, record expiration patterns, and their limitations.	https://surrealdb.com/docs/learn/schema-management/events-and-triggers/reactive-patterns
Time-series model — Storing event histories, measurements, and timestamped records.	https://surrealdb.com/docs/learn/data-models/time-series/overview
Time-series aggregation — Time-based aggregation and analytical querying.	https://surrealdb.com/docs/learn/data-models/time-series/aggregation-queries
Time buckets and windowing — Organizing and querying data in temporal windows.	https://surrealdb.com/docs/learn/data-models/time-series/time-buckets-and-windowing
File storage — Working with files and file references in SurrealDB.	https://surrealdb.com/docs/learn/schema-management/files/working-with-files
Buckets — Defining and managing file-backed storage buckets.	https://surrealdb.com/docs/learn/schema-management/files/buckets
Security architecture — Overall model for database authentication and authorization.	https://surrealdb.com/docs/learn/security
Authentication methods — Credentials and authentication options at different database scopes.	https://surrealdb.com/docs/learn/security/authentication/overview
Permissions and row-level security — Declarative access control at table and record levels.	https://surrealdb.com/docs/learn/security/authorization/permissions-and-row-level-security
Capabilities — Restricting database functions and execution capabilities, particularly for untrusted workloads.	https://surrealdb.com/docs/learn/security/authorization/capabilities
Tokens and JWTs — Authentication-token configuration and validation.	https://surrealdb.com/docs/learn/security/authorization/tokens-and-jwts
Security best practices — Production hardening and deployment security considerations.	https://surrealdb.com/docs/learn/security/best-practices/security-best-practices
Important lifecycle constraint: SurrealDB currently has no native record-level TTL setting or built-in scheduled cleanup mechanism. Expiration therefore requires deliberate application-side or event-based handling. For transient agent analysis, testing state, and long-term results sharing one persistence system, agents should pay particular attention to the Reactive patterns guide. 

Database



6. SDKs, protocols, and interoperability
SurrealDB has a broad set of official language SDKs and multiple query interfaces. Agents should consult the SDK-specific reference alongside the server documentation to avoid confusing features supported by a database version with those exposed by a particular SDK.
Resource / Topic	Full URL
SDK directory — Overview of official SDKs and supported programming languages.	https://surrealdb.com/docs/languages
Rust SDK — Native Rust API, embedded database support, transactions, typed values, and asynchronous operations.	https://surrealdb.com/docs/reference/rust
Rust embedding — In-process SurrealDB integration, engine features, and local connections.	https://surrealdb.com/docs/reference/rust/embedding
Python SDK — Python connection models, synchronous/asynchronous operations, transactions, and value types.	https://surrealdb.com/docs/reference/python
JavaScript / TypeScript SDK — Browser, Node.js, WebAssembly, streaming, and typed client access.	https://surrealdb.com/docs/reference/javascript
Go SDK — Native Go connections and database operations.	https://surrealdb.com/docs/reference/golang
.NET SDK — C# and .NET database integration, authentication, streaming, and data manipulation.	https://surrealdb.com/docs/reference/dotnet
Java SDK — JVM client capabilities and integration.	https://surrealdb.com/docs/reference/java
Kotlin SDK — Kotlin client models, serialization, live queries, and transactions.	https://surrealdb.com/docs/reference/kotlin
PHP SDK — PHP client reference, including version-specific APIs.	https://surrealdb.com/docs/reference/php
Swift SDK — Swift client access and supported data types.	https://surrealdb.com/docs/reference/swift
Mojo SDK — Mojo client capabilities, query builders, and database integration.	https://surrealdb.com/docs/reference/mojo
Community SDKs — Third-party and experimental language integrations.	https://surrealdb.com/docs/languages/community
HTTP API — Direct REST/HTTP access for database operations and scripting.	https://surrealdb.com/docs/reference/rest-api/http-protocol
RPC protocol — Database RPC access over supported transports, including WebSocket.	https://surrealdb.com/docs/reference/rest-api/rpc-protocol
CBOR protocol — Binary encoding, custom tags, and data interchange semantics.	https://surrealdb.com/docs/reference/rest-api/cbor-protocol
PostgreSQL wire protocol — Using compatible PostgreSQL clients and drivers to connect to SurrealDB, with documented limitations.	https://surrealdb.com/docs/reference/rest-api/postgres-protocol
GraphQL interface — Querying SurrealDB through GraphQL.	https://surrealdb.com/docs/learn/querying/graphql/overview
ISO GQL interface — Graph Query Language interface, syntax, and graph querying.	https://surrealdb.com/docs/learn/querying/gql/overview
For Rust- and Python-heavy applications, the Rust and Python SDKs deserve dedicated review. The PostgreSQL wire protocol is also significant for interoperability with existing database clients, although protocol compatibility should not be interpreted as full PostgreSQL behavioral compatibility. 

SurrealDB
+1



7. Developer tooling, schema migrations, testing, and IDE integration
High-priority tooling for agent-driven development
This is one of the most valuable sections to expose to agents because it contains capabilities that can replace custom schema-management scripts, manual syntax checking, migration logic, and portions of bespoke validation code.
Resource / Topic	Full URL
SurrealDB CLI overview — Available command-line tools and their purposes.	https://surrealdb.com/docs/reference/cli
SurrealDB CLI command reference — Full listing of server, database, import/export, validation, and other commands.	https://surrealdb.com/docs/reference/cli/surrealdb-cli/commands
SurrealQL CLI / REPL — Execute statements interactively or pipe SQL scripts into the database.	https://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/sql
SurrealQL validation — Validate SurrealQL syntax from the command line without executing the statements.	https://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/validate
SurrealQL formatter — Canonical formatting and the dedicated surqlfmt tool.	https://surrealdb.com/docs/reference/cli/formatter/overview
SurrealKit overview — Official schema-as-code and database migration tooling.	https://surrealdb.com/docs/manage/schema-migration
Schema synchronization — Reconcile database definitions against source-controlled .surql files.	https://surrealdb.com/docs/manage/schema-migration/sync
Migration rollouts — Reviewed migrations, phased application, and rollback mechanics.	https://surrealdb.com/docs/manage/schema-migration/rollouts
Schema modules and targets — Manage distinct schemas across multiple databases or environments.	https://surrealdb.com/docs/manage/schema-migration/modules-and-targets
Seeding — Apply and track database seed data.	https://surrealdb.com/docs/manage/schema-migration/seeding
Type generation — Generate schema-derived types and structured representations.	https://surrealdb.com/docs/manage/schema-migration/typegen
Database testing — Declarative testing of schemas, permissions, and API behavior.	https://surrealdb.com/docs/manage/schema-migration/testing
CI/CD integration — Database validation and schema testing in automated development pipelines.	https://surrealdb.com/docs/manage/schema-migration/testing/ci-cd
SurrealKit source code — Rust implementation, issues, releases, and examples.	https://github.com/surrealdb/surrealkit
SurrealDB Studio — Official visual database interface, query editor, and database exploration environment.	https://surrealdb.com/docs/explore/studio
Studio web application — Browser-based access to the SurrealDB Studio interface.	https://studio.surrealdb.com
SurrealQL language server — Editor tooling and language-aware development support.	https://github.com/surrealdb/surrealql-language-server
SurrealQL Tree-sitter grammar — Structured parsing infrastructure for editors, syntax tooling, and agent-based code intelligence.	https://github.com/surrealdb/surrealql-tree-sitter
VS Code integration — SurrealQL editor integration and language tooling.	https://github.com/surrealdb/surrealql-vsx
Neovim integration — SurrealQL support for Neovim.	https://github.com/surrealdb/surrealql-neovim
Two details deserve emphasis.
First, SurrealKit supports both synchronization and controlled migrations. Synchronization is intended for rapid development and disposable environments; rollouts target environments where schema changes require review and preservation of existing data. Its current 1.0 documentation refers to a prerelease, while the stable installation line remains 0.7, so agents should verify the installed version before using 1.0-specific commands. 

Database



Second, Tree-sitter, the language server, the formatter, and CLI validation serve different purposes. Together, they make it possible to separate structured query construction, syntactic verification, formatting, and actual execution testing.
8. Storage engines, deployment architecture, and observability
Resource / Topic	Full URL
Storage engines — Compare supported backends and their operational characteristics.	https://surrealdb.com/docs/build/embedding/storage-engines
Embedding overview — Run SurrealDB directly inside applications rather than as a separate service.	https://surrealdb.com/docs/build/embedding
Rust embedded deployments — Configure the embedded engine from Rust applications.	https://surrealdb.com/docs/build/embedding/by-language/rust
Python embedded deployments — Supported embedded usage from Python.	https://surrealdb.com/docs/build/embedding/by-language/python
Server installation — Installation options across operating systems.	https://surrealdb.com/docs/running/installation
In-memory deployment — Memory-backed database instances and persistence options.	https://surrealdb.com/docs/running/in-memory
File-backed deployment — Single-node persistent database deployment.	https://surrealdb.com/docs/running/file-backed
Docker deployment — Run SurrealDB with containers.	https://surrealdb.com/docs/running/docker
Deployment models — Detailed comparison of embedded, single-node, and distributed architectures.	https://surrealdb.com/docs/manage/self-hosted/deployment-models
Self-hosted operations — Infrastructure-level hosting and operational configuration.	https://surrealdb.com/docs/manage/self-hosted
Kubernetes — Container orchestration and cluster deployment guidance.	https://surrealdb.com/docs/manage/self-hosted/kubernetes
Backup and recovery — Recovery and operational backup mechanisms for self-hosted deployments.	https://surrealdb.com/docs/manage/self-hosted/backups-and-recovery
Configuration — Database configuration options for self-hosted environments.	https://surrealdb.com/docs/manage/self-hosted/configuration
Environment variable reference — Available SURREAL_ configuration variables.	https://surrealdb.com/docs/reference/cli/surrealdb-cli/environment-variables
Cloud instances — Managed-instance provisioning, scaling, and lifecycle operations.	https://surrealdb.com/docs/manage/instances
SurrealCTL — Administrative CLI for managing SurrealDB Cloud resources.	https://surrealdb.com/docs/manage/surrealctl
Observability overview — Metrics, logging, telemetry, and production diagnostics.	https://surrealdb.com/docs/manage/observability
OpenTelemetry integration — Telemetry emission using OTLP and associated configuration.	https://surrealdb.com/docs/manage/observability/telemetry
Metrics and Prometheus — Database metrics and collection patterns.	https://surrealdb.com/docs/manage/observability/observability
Slow-query logging — Identify and investigate expensive queries.	https://surrealdb.com/docs/manage/observability/slow-query-logging
Audit logging — Activity auditing and available logging capabilities.	https://surrealdb.com/docs/manage/observability/audit-logging
SurrealKV source — SurrealDB's own storage engine implementation and development history.	https://github.com/surrealdb/surrealkv
A key storage distinction for the current release is that RocksDB remains the recommended production engine for single-node, on-disk servers running SurrealDB 3.3, whereas SurrealKV is still classified as beta at that server version. SurrealKV is particularly worth evaluating for embedded use. SurrealMX provides an in-memory backend with optional persistence capabilities, but it has different support for versioned queries.
These considerations matter when deciding whether scratch workloads, durable results, and long-running analytical processes should share an engine, database, or instance. 

Database



9. Data ingestion, migration, synchronization, and ecosystem integrations
This group is relevant for building pipelines that ingest data from external systems, continuously synchronize changes, and integrate SurrealDB into existing infrastructure.
Resource / Topic	Full URL
Migration overview — Official entry point for importing and migrating data into SurrealDB.	https://surrealdb.com/docs/build/migrating
Surreal Sync — Rust-based tool supporting full imports and incremental synchronization from external data sources.	https://github.com/surrealdb/surreal-sync
PostgreSQL migration — Source-specific migration concepts and tooling.	https://surrealdb.com/docs/build/migrating/from-other-databases/from-postgresql
MongoDB migration — Document import and data-model conversion.	https://surrealdb.com/docs/build/migrating/from-other-databases/from-mongodb
Neo4j migration — Graph data migration and relationship preservation.	https://surrealdb.com/docs/build/migrating/from-other-databases/from-neo4j
MySQL migration — Importing relational data from MySQL.	https://surrealdb.com/docs/build/migrating/from-other-databases/from-mysql
Snowflake migration — Ingesting data from analytical warehouse systems.	https://surrealdb.com/docs/build/migrating/from-other-databases/from-snowflake
Kafka ingestion — Stream and event ingestion through Kafka.	https://surrealdb.com/docs/build/migrating/from-files-and-streams/kafka
CSV ingestion — Importing tabular data.	https://surrealdb.com/docs/build/migrating/from-files-and-streams/csv
JSON Lines ingestion — Importing structured record streams and large record collections.	https://surrealdb.com/docs/build/migrating/from-files-and-streams/json-lines
SurrealDB 2.x → 3.x migration — Breaking changes, compatibility adjustments, and upgrade considerations.	https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/2x-to-3x
SurrealDB 3.2 → 3.3 migration — Release-specific compatibility and behavioral changes.	https://surrealdb.com/docs/build/migrating/from-old-surrealdb-versions/32-to-33
Integration directory — Overview of supported third-party systems.	https://surrealdb.com/docs/build/integrations
Data management integrations — Ecosystem tools for ingestion, synchronization, and automation.	https://surrealdb.com/docs/build/integrations/data-management/overview
Airbyte — Data integration and extraction/load workflows.	https://surrealdb.com/docs/build/integrations/data-management/airbyte
Fivetran — Managed data pipeline integration.	https://surrealdb.com/docs/build/integrations/data-management/fivetran
n8n — Workflow automation connected to SurrealDB.	https://surrealdb.com/docs/build/integrations/data-management/n8n
Beekeeper Studio — General-purpose database client integration.	https://surrealdb.com/docs/build/integrations/database-clients/beekeeper-studio
Maturity consideration: Surreal Sync supports an unusually broad range of sources, including CDC-based flows, but its repository explicitly describes it as under active development and not yet stable. It should be evaluated accordingly before being made a critical production dependency. 

surrealdb/surreal-sync · GitHub



10. Agent-specific infrastructure, MCP, and AI integrations
Especially relevant alongside Context7
SurrealDB has first-party integration mechanisms expressly targeted at coding agents. These provide functionality beyond ordinary documentation retrieval.
Resource / Topic	Full URL
Agent setup overview — Central directory for integrating coding agents with SurrealDB.	https://surrealdb.com/docs/agents
Codex integration — Setup for OpenAI Codex, including skills and MCP access.	https://surrealdb.com/docs/agents/codex
Claude Code integration — Agent-specific configuration instructions.	https://surrealdb.com/docs/agents/claude-code
Cursor integration — Agent and IDE configuration for Cursor.	https://surrealdb.com/docs/agents/cursor
VS Code integration — Agent-oriented editor configuration.	https://surrealdb.com/docs/agents/vscode
AI-agent integration overview — Available MCP servers, Agent Skills, and framework integrations.	https://surrealdb.com/docs/build/ai-agents
Official Agent Skills — Packaged knowledge for writing SurrealQL and working with supported SurrealDB capabilities.	https://surrealdb.com/docs/build/ai-agents/agent-skills
Agent Skills source — Inspect, install, or adapt SurrealDB's first-party coding-agent skills.	https://github.com/surrealdb/agent-skills
SurrealDB MCP overview — Available tools and capabilities of the hosted MCP server.	https://surrealdb.com/docs/build/ai-agents/mcp
Hosted MCP endpoint — Remote endpoint primarily for SurrealDB Cloud integrations.	https://mcp.surrealdb.com
Embedded MCP server — Locally hosted MCP via surreal mcp or the server's /mcp endpoint.	https://surrealdb.com/docs/build/ai-agents/mcp/embedded
MCP CLI reference — Command-line options for running the built-in MCP server.	https://surrealdb.com/docs/reference/cli/surrealdb-cli/commands/mcp
MCP usage examples — Example agent operations and workflows.	https://surrealdb.com/docs/build/ai-agents/mcp/examples
Agent integration rules — Guidance for AI-assisted development using SurrealDB integrations.	https://surrealdb.com/docs/build/integrations/agent-rules
AI framework directory — Available integration guides for agent frameworks.	https://surrealdb.com/docs/build/integrations/ai-frameworks/overview
LangChain — Integration with chains and retrieval workflows.	https://surrealdb.com/docs/build/integrations/ai-frameworks/langchain
LangGraph — Integration with graph-structured AI-agent workflows.	https://surrealdb.com/docs/build/integrations/ai-frameworks/langgraph
LlamaIndex — Integration with document indexing and retrieval pipelines.	https://surrealdb.com/docs/build/integrations/ai-frameworks/llamaindex
Pydantic AI — Typed Python agent integration.	https://surrealdb.com/docs/build/integrations/ai-frameworks/pydantic-ai
CocoIndex — AI data indexing and processing integration.	https://surrealdb.com/docs/build/integrations/ai-frameworks/cocoindex
Embedding providers — Integrating model-generated vector embeddings into SurrealDB.	https://surrealdb.com/docs/build/integrations/embeddings-providers/overview
FastEmbed — Local embedding generation integration.	https://surrealdb.com/docs/build/integrations/embeddings-providers/fastembed
How these complement Context7
Mechanism	Distinct contribution
Context7	Up-to-date library documentation and API usage patterns.
SurrealDB llms.txt and Markdown documentation	Canonical, comprehensive database documentation, including newly released and version-specific functionality.
SurrealDB Agent Skills	Packaged coding-agent guidance for SurrealQL, vector search, and the Python SDK.
SurrealDB MCP	Direct inspection of schemas and execution of queries against actual database instances.
SurrealKit, formatter, and CLI validation	Mechanical verification of query syntax, schema definitions, and migration behavior.
There is one important MCP distinction: surreal mcp over stdio operates against an embedded database with owner-level access, whereas the HTTP /mcp endpoint can use normal database authentication and permissions. For agents interacting with existing data, the latter provides substantially better scope control. 

Database
+1



11. SurrealDB Agent Memory: knowledge, provenance, and agent state
SurrealDB Agent Memory is a separate, higher-level product built around persistent memory and knowledge for AI agents. It is worth including because it exposes concepts and tooling that may be useful when designing long-lived code intelligence and agent execution histories.
Resource / Topic	Full URL
Agent Memory overview — Product architecture, major capabilities, and entry points.	https://surrealdb.com/docs/agent-memory
Architectural principles — Goals, boundaries, and underlying design philosophy.	https://surrealdb.com/docs/agent-memory/architecture/principles-and-goals
Unified memory and knowledge model — Two-layer architecture for authoritative and experiential knowledge.	https://surrealdb.com/docs/agent-memory/mental-model/two-layer-architecture
Contexts and scope — Context isolation, tenant boundaries, and scoping of stored knowledge.	https://surrealdb.com/docs/agent-memory/mental-model/contexts-and-scope
Provenance and traceability — Attaching evidence and origin information to knowledge records.	https://surrealdb.com/docs/agent-memory/mental-model/provenance-and-traceability
Tri-temporal model — Valid time, system time, supersession, and temporal knowledge evolution.	https://surrealdb.com/docs/agent-memory/architecture/tri-temporal-model
Memory lifecycle — Supersession, decay, and forgetting.	https://surrealdb.com/docs/agent-memory/mental-model/memory-lifecycle
Hybrid retrieval — Combining vector retrieval, BM25, and graph-aware relevance.	https://surrealdb.com/docs/agent-memory/retrieve/hybrid-search
Graph traversal — Retrieval over structurally linked knowledge.	https://surrealdb.com/docs/agent-memory/retrieve/graph-traversal
Reconciliation and supersession — Reconciling conflicting information and evolving knowledge.	https://surrealdb.com/docs/agent-memory/reasoning/reconciliation-and-supersession
Forget operations — Scoped deletion, forgetting, and erasure semantics.	https://surrealdb.com/docs/agent-memory/operations/forget
Caching and invalidation — Semantic response cache behavior and invalidation.	https://surrealdb.com/docs/agent-memory/tuning/caching-and-invalidation
Agent Memory MCP installation — Connect AI agents to the memory-specific MCP tool surface.	https://surrealdb.com/docs/agent-memory/integrations/mcp-server/install
Agent Memory MCP tools — Tool descriptions and invocation contracts.	https://surrealdb.com/docs/agent-memory/reference/mcp-tools
Python SDK — Programmatic interaction with Agent Memory from Python.	https://surrealdb.com/docs/agent-memory/reference/sdk-python
Coding-agent memory example — Worked approach for adding persistent project memory to a coding agent.	https://surrealdb.com/docs/agent-memory/cookbooks/build/coding-agent-with-project-memory
Agent integration guide — AGENTS.md-oriented guidance for coding-agent integrations.	https://surrealdb.com/docs/agent-memory/reference/agents
The most interesting components here are the unified authority model, provenance, temporal validity, reconciliation, and lifecycle management. These can be studied as architectural patterns even if the underlying application continues to use core SurrealDB directly rather than adopting Agent Memory as an additional service. 

SurrealDB



12. Extensibility, source code, examples, and deeper technical references
These provide implementation-level evidence and additional capabilities outside the main query and administration interfaces.
Resource / Topic	Full URL
Core SurrealDB repository — Rust implementation, query engine, storage abstractions, protocol support, issues, and development history.	https://github.com/surrealdb/surrealdb
Core release history — Tagged releases and source-level change history.	https://github.com/surrealdb/surrealdb/releases
Python SDK source — Implementation details for Python API behavior and version compatibility.	https://github.com/surrealdb/surrealdb.py
JavaScript SDK source — TypeScript implementation and client internals.	https://github.com/surrealdb/surrealdb.js
Extensions overview — Mechanisms for extending SurrealDB functionality.	https://surrealdb.com/docs/learn/extensions
WASM plugins — Loading and executing custom WebAssembly modules.	https://surrealdb.com/docs/learn/extensions/guides/using-wasm-plugins
Creating custom modules — Building custom database modules and extensions.	https://surrealdb.com/docs/learn/extensions/guides/creating-custom-modules
Module architecture — Extension architecture and execution boundaries.	https://surrealdb.com/docs/learn/extensions/guides/module-architecture
Surrealism plugins — Rust-to-WASM plugins callable from SurrealQL.	https://surrealdb.com/docs/learn/extensions/plugins/overview
SurrealML — Machine-learning model storage and execution capabilities.	https://surrealdb.com/docs/explore/ml-models
SurrealDB learning resources — Tutorials, courses, and educational material.	https://surrealdb.com/learn
SurrealDB book — Narrative documentation and examples for deeper understanding.	https://surrealdb.com/learn/book
Graph RAG tutorial — End-to-end example integrating graph data and retrieval-augmented generation.	https://surrealdb.com/docs/explore/tutorials/tutorials/gen-ai-chatbot
Knowledge graph tutorial — Building graph-based knowledge representations.	https://surrealdb.com/docs/explore/tutorials/tutorials/how-to-build-a-knowledge-graph-for-ai
Rust semantic-search tutorial — Implementing vector search using Rust and SurrealDB.	https://surrealdb.com/docs/explore/tutorials/tutorials/semantic-search-in-rust
GitHub Actions example — Incorporating SurrealDB into automated development workflows.	https://surrealdb.com/docs/explore/tutorials/tutorials/github-actions

Recommended approach for your programming agents
Given the breadth of this ecosystem, I would not instruct an agent to load every linked resource into context. Instead, I'd make the full directory available for discovery, then establish a small set of canonical entry points and a retrieval hierarchy.
1. Documentation discovery and authoritative behavior
Use https://surrealdb.com/docs/llms.txt as the canonical source index. Resolve the relevant documentation pages and fetch their Markdown variants. Treat installed-version compatibility as mandatory, particularly around SurrealDB 3.x functionality.
2. Native tools before bespoke implementation
Prefer the documented capabilities of SurrealKit, SurrealQL validation, the formatter, EXPLAIN, database-side schema constraints, and database-native graph/search functionality before creating parallel infrastructure in application code.
3. Source-level verification when necessary
Use the official Rust repository, SDK implementations, SurrealKV source, and SurrealKit source when documentation is incomplete or when exact execution semantics matter. Source inspection is particularly useful for concurrency, transaction boundaries, index behavior, and embedded database characteristics.
4. Operational distinctions
Explicitly distinguish embedded versus server operation, RocksDB versus SurrealKV, development synchronization versus controlled rollouts, and ephemeral state versus retained records. Do not assume capabilities such as TTL, background scheduling, or unrestricted cross-version storage compatibility exist merely because they are typical database features.
5. Agent interaction
Use Context7 for library APIs, official Agent Skills for SurrealDB-specific programming conventions, and MCP for inspecting the actual database and schema. Keep direct database access appropriately scoped and distinguish read-only inspection from mutations.
Highest-value subset
If you want to provide an agent with a smaller starting bundle rather than the entire reference, these eight sources cover most of the discovery and implementation surface:
Complete documentation index
https://surrealdb.com/docs/llms.txt

 Copy
SurrealQL language reference
https://surrealdb.com/docs/reference/query-language

 Copy
Core data models
https://surrealdb.com/docs/learn/data-models

 Copy
Storage engines
https://surrealdb.com/docs/build/embedding/storage-engines

 Copy
SurrealKit schema and migrations
https://surrealdb.com/docs/manage/schema-migration

 Copy
CLI and developer tooling
https://surrealdb.com/docs/reference/cli

 Copy
Agent Skills and MCP
https://surrealdb.com/docs/agents

 Copy
Implementation source
https://github.com/surrealdb/surrealdb

 Copy

These eight sources are sufficient to bootstrap agent discovery, while the complete directory above provides deeper references for specific capabilities, integrations, and operational decisions.