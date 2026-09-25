# library-context

Compile pinned Python libraries into evidence-carrying behavioral models for coding agents.
Rust extraction, Arrow contracts, DataFusion derivation and Delta publication support queryable
facts and bounded semantic analyses; Python/FastMCP exposes the serving tools.

The project is in active design and implementation. [STATUS](STATUS.md) owns the current
checkpoint and remaining qualification. Accepted architecture includes labeled targets.

- [Documentation and task routes](docs/README.md)
- [Architecture map](docs/design/README.md)
- [Development instructions](AGENTS.md)
- [Decision history](docs/adr/README.md)

## Documentation locally

Run `just bootstrap-docs` once to install the declared documentation tools and cache isolated test
dependencies. Then `just docs-check` builds and checks the complete static artifact under
`build/docs/site/`. `just docs-test` runs focused publisher and ADR tests. `just docs-serve` builds
and serves on http://127.0.0.1:8000. Rebuild after edits; the preview is not a watcher.

These commands use the pinned Python interpreter without installing native product packages.
Source Markdown remains usable without a site. CI produces a downloadable site artifact; public
hosting is not configured. See [publishing operations](docs/publishing.md) for details.
