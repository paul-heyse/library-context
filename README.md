# library-context

Target: compile pinned Python libraries into an API and evidence catalog that helps coding agents
find, configure and use built-in features with precise, inspectable support. Existing Rust facts and
bounded behavioral analyses, Arrow/DataFusion/Delta and PostgreSQL serving provide the foundation.
The [replacement product design](docs/design/sections/api-and-evidence-product.md) is an accepted
target; its new API/evidence capabilities and Context7 differentiation remain to be implemented
and measured.

The project is in active design and implementation. [STATUS](STATUS.md) owns the current
checkpoint and remaining qualification. Accepted architecture includes labeled targets.

- [Documentation and task routes](docs/README.md)
- [Architecture map](docs/design/README.md)
- [Development instructions](AGENTS.md)
- [Decision records (current rationale)](docs/adr/README.md)

## Documentation locally

Run `just bootstrap-docs` once to install the declared documentation tools and cache isolated test
dependencies. Then `just docs-check` builds and checks the complete static artifact under
`build/docs/site/`. `just docs-test` runs focused publisher and ADR tests. `just docs-serve` builds
and serves on http://127.0.0.1:8000. Rebuild after edits; the preview is not a watcher.

These commands use the pinned Python interpreter without installing native product packages.
Source Markdown remains usable without a site. CI produces a downloadable site artifact; public
hosting is not configured. See [publishing operations](docs/publishing.md) for details.

PostgreSQL-backed compiles use the [local PostgreSQL setup and recovery commands](docs/postgresql.md).
SQLx owns cache, operational services and immutable serving projections in Rust; Delta retains
canonical publication and offline replay. MCP serves an explicitly pinned ready PostgreSQL
generation. Portable bundles remain export/reference/recovery inputs. See the runbook for
`lctx serving import/select` and `lctx-mcp --library NAME`.
