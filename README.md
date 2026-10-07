# library-context

Target: compile pinned Python libraries into an API and evidence catalog that helps coding agents
find, configure and use built-in features with precise, inspectable support. Existing Rust facts and
bounded behavioral analyses provide enrichment. The typed Rust model owns semantic contracts,
SurrealDB hosts exact persisted compiler inputs and semantic graphs, and Arrow/DataFusion supplies
spillable in-process compute. Native publication and serving operate on immutable snapshots.
The [persisted execution pivot](docs/plans/persisted-graph-execution-plan_2026-10-07.md) is being
integrated and functionally verified; STATUS owns its acceptance boundary.
The [replacement product design](docs/design/sections/api-and-evidence-product.md) is an accepted
target. Catalog compilation and ordered API contracts are implemented, with behavioral enrichment
available explicitly. Surface/configuration, contextual evidence and typed selection remain in
the forward plan; Context7 differentiation is not yet measured.

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

Compilation uses an authenticated persistent SurrealDB runtime configured by `--runtime-config`.
`lctx compile <library> --through facts|normalized|analysis|catalog --profile catalog|behavioral`
returns an unselected native snapshot. Add `--artifact-only --output <directory>` to export the
complete graph, originals and retained compiler state from the same native compiler instead.
External `publish-artifact` independently admits the current format. See the
[SurrealDB runbook](docs/surrealdb.md) for configuration and explicit selection.
Operator activation and real-library pilots remain separate from owned fixture acceptance.
