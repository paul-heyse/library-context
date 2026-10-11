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

## Tests

Run `just test-all` for the complete ordinary repository test scope: workspace Rust unit and
integration tests, Python tests, Rust doctests and compile-fail controls. Native boundaries borrow
the installed persistent validation service with logical isolation; the command supplies their
configuration and retains ownership through the existing fixture lifecycle. It checks this checkout's
Python/native prerequisites and reports missing prerequisites as `blocked`, with the repair command.
Environment synchronization, service installation and migrations remain explicit operations.

`just test-all --print` previews the commands, readiness and coverage without building tests.
`just test-all --live` follows output; full logs and `summary.json` stay under the run reported by
the command. Independent boundaries continue after failures; `just verify --rerun RUN` repeats
failed, blocked and unreached boundaries with their original scope and compilation profile.
Compiler and test parallelism use their normal available settings. The current default Cargo
profile is release pending local-profile qualification; `py314` is the Python/Ruff version target.

The MCP journey uses fresh producer/consumer processes without restarting the shared daemon.
`just test-all --restart-service` explicitly adds the existing drained maintenance/restart check;
use it when that disruptive persistence assurance is intended. The same option applies to
`just qualify --restart-service`.

Coverage accounting compares the resolved plan with Cargo workspace targets and pytest-configured
test files, and refuses unassigned ordinary targets or files. New tests inside an already selected
target or directory are included automatically. Default-feature tests are covered; optional-feature
combinations are listed separately. Ignored controls and their reasons are visible in the coverage
report: the two fixture-only ignored native controls run explicitly, joined-process oracle children
remain owned by their parents, and maintenance, historical-format and retained-fixture-only controls
remain excluded by default. An ordinary-suite pass does not qualify those excluded scenarios.

`just qualify` runs the same complete ordinary test scope plus Clippy, dependency, documentation
and other non-functional leaves. Bare `cargo nextest run --workspace --no-fail-fast` covers Rust
tests only and does not acquire native fixture configuration; use `just test-all` for the integrated
repository scope or `just fixture -- cargo nextest run …` for a fixture-backed bare Rust selection.

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
