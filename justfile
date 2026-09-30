# The only command surface agents need. `just` lists recipes.
# Check recipes do not edit source; tests may create their own data fixtures.
# `fmt` and `adr new|supersede|index` edit the working tree.

set shell := ["python3", "scripts/build_environment.py", "--", "bash", "-euo", "pipefail", "-c"]

# List recipes
default:
    @just --list

# Apply the shared library-skill selection for Codex and Claude Code
skills-sync:
    python3 scripts/library_skills.py

# Check selected library links without changing files
skills-check:
    python3 scripts/library_skills.py --check

# The default loop: format check, lints, optimized cached core tests, rules, ADRs, agent config
check: fmt-check lint test py-check rules-scan rules-test lint-agents
    uv run python scripts/adr.py lint

# The SQLx offline check returns with serving (cutover phase 5, T12); the dormant serving queries
# are frozen in `.sqlx`.
# Everything: check + fixtures + dependency policy + real PostgreSQL
test-all: check fixtures-check deps gold test-postgres

# rustfmt + ruff format check (no changes)
fmt-check:
    cargo fmt --check
    uv run ruff format --check

# Format everything (mutating)
fmt:
    # Virtual-root defaults cover workspace members; --all would also rewrite vendored patches.
    cargo fmt
    uv run ruff format
    uv run ruff check --fix --quiet

# clippy (deny warnings) + ruff lint
lint:
    cargo clippy --release --workspace --all-targets --quiet -- -D warnings
    uv run ruff check --quiet

# Core Rust tests share the release profile with the shipped binary. Cargo rebuilds only changed
# Rust inputs; repeated runs execute cached optimized test binaries. Tests own their data state.
# Snapshots never auto-accept (INSTA_UPDATE=no).
test *args:
    INSTA_UPDATE=no cargo nextest run --release --workspace --no-fail-fast --no-tests=pass {{args}}

# Python tests (scripts and lctx_mcp over the fixture generation) + pyrefly
py-check:
    uv run pytest
    uv run pyrefly check --summary=none

# Independent runtime challenges of the release producer (S7): CPython admission of flow facts
# and of served claims over compiled generated packages. Also part of `py-check`.
oracles:
    uv run pytest tests/scripts/test_flow_soundness.py tests/scripts/test_semantic_soundness.py -q

# `structured-eval`, `score` and `ranking-check` read served generations; they return with serving
# (cutover phase 5).

# Pinned-family single-version check + cargo-deny sources/licenses + the Pyrefly fork (ADR-0046)
deps:
    uv run python scripts/check_family.py Cargo.lock
    cargo deny --log-level error check bans sources licenses
    uv run python scripts/check_pyrefly_fork.py
    # A dependency no crate uses pins nothing (H1 O2).
    cargo shear --exclude lctx-workspace-hack
    cargo hakari generate --diff
    cargo hakari manage-deps --dry-run

# Regenerate the executable's dependency feature union after dependency changes (ADR-0079).
build-features:
    cargo hakari generate
    cargo hakari manage-deps -y

# The gold reference and the analyzed library name one FastMCP (ADR-0046). Separate from `deps`,
# so a skill refresh in progress never reads as a dependency-family break (ADR-0002's trigger).
gold:
    uv run python scripts/check_gold.py

# Byte-compile Python fixtures (input data) so they cannot silently become syntax-error cases
fixtures-check:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(python3 scripts/build_environment.py --shell)"
    mapfile -t files < <(find fixtures/python -name '*.py' -not -path '*/_invalid/*')
    [ ${#files[@]} -eq 0 ] && { echo "fixtures-check: not_run (no fixtures yet)"; exit 0; }
    uv run python -c 'import ast,sys; [ast.parse(open(f,"rb").read(), f) for f in sys.argv[1:]]' "${files[@]}"
    echo "fixtures-check: ${#files[@]} files parse"

# `just pilot` (the Delta compile, serving import and MCP smoke) is retired with the old pipeline
# (plan P1.1). The facts pilots for both profiles return at plan Q.

# The embedding service (DESIGN §11.1, ADR-0080): the gpu-stack SM120 wheel from services/vllm
# project, serving Qwen3-Embedding-8B at its pinned revision on the local GPU
embed-serve port="8000":
    uv run python scripts/embed_serve.py --port {{port}}

# The live leg of the client conformance check (§11.1, E2): needs `just embed-serve` running
embed-conformance url="http://127.0.0.1:8000":
    LCTX_EMBED_URL={{url}} LCTX_CONFORMANCE_OUT="$PWD/build/conformance-rust.json" cargo nextest run --release -p lctx-embed -E 'test(live_conformance_vectors)' --status-level none --final-status-level fail
    uv run python scripts/embed_conformance.py build/conformance-rust.json --url {{url}}

# ast-grep scan over the tree (rules/ grows from design-review findings)
rules-scan:
    @if ls rules/*.yml >/dev/null 2>&1; then ast-grep scan; else echo "rules-scan: not_run (no rules yet)"; fi

# ast-grep rule fixtures
rules-test:
    @if ls rules/*.yml >/dev/null 2>&1; then ast-grep test --skip-snapshot-tests; else echo "rules-test: not_run (no rules yet)"; fi

# ADR tooling: new <slug> [--title T] | supersede <ADR-NNNN> <slug> | index | lint | revisit
[positional-arguments]
adr *args:
    @uv run --no-project --offline --no-python-downloads python scripts/adr.py "$@"

# Claude/Codex parity and dead-reference check for agent instructions and skills
lint-agents:
    uv run --no-project --offline --no-python-downloads python scripts/check_agents.py

# Tool presence and versions (compare with docs/pins.md)
doctor:
    #!/usr/bin/env bash
    eval "$(python3 scripts/build_environment.py --shell)"
    for t in cargo rustc cargo-nextest cargo-insta cargo-deny cargo-shear cargo-hakari uv ast-grep rg git gh clang clang++ llvm-config mold sccache sqlx psql pg_dump pg_restore docker; do
      printf '%-14s ' "$t"; command -v "$t" >/dev/null && "$t" --version 2>/dev/null | head -1 || echo MISSING
    done
    printf '%-14s ' ruff; uv run ruff --version
    printf '%-14s ' pyrefly; uv run pyrefly --version
    printf '%-14s ' python; uv run python --version
    uv run --no-project --offline --no-python-downloads python scripts/docs.py doctor

# Isolated build measurements: `preflight`, `capture <dir>`, `run <dir> --variant ...`, `report <dir>`.
# `run` is the only subcommand that compiles the Rust workspace.
[positional-arguments]
bench-builds *args:
    @uv run --no-sync python scripts/build_measurements.py "$@"

# Install declared documentation binaries and isolated test dependencies (network allowed)
bootstrap-docs:
    uv run --no-project python scripts/docs.py bootstrap

# Build Markdown, Pagefind and offline link/fragment checks without product dependencies
docs:
    uv run --no-project --offline --no-python-downloads python scripts/docs.py build

# Existing metadata checks plus a complete publication
docs-check:
    uv run --no-project --offline --no-python-downloads python scripts/docs.py check

# Focused documentation/ADR regressions only
docs-test:
    uv run --no-project --offline --no-python-downloads python scripts/docs.py test

# Build then serve the finished artifact; no watcher or reindex during serving
docs-serve port="8000":
    uv run --no-project --offline --no-python-downloads python scripts/docs.py serve --port {{port}}

# Fetch the exact disposable PG18 image explicitly before database qualification.
postgres-test-setup:
    docker pull "postgres:$(cat specs/postgres-image.txt)"
    docker pull "$(cat specs/postgres-vector-image.txt)"

# The serving suites are dormant until cutover phase 5.
# Real PostgreSQL 18 (disposable containers): generation store, provider sessions, CLI, transition
test-postgres:
    @docker image inspect "$(cat specs/postgres-vector-image.txt)" >/dev/null || { echo 'blocked: run just postgres-test-setup'; exit 2; }
    INSTA_UPDATE=no cargo nextest run --release -p lctx-postgres -p cpg-extract -p cpg-core -p lctx --no-fail-fast -E 'not binary(serving)'
    cargo build --release -p lctx
    LCTX_POSTGRES_TEST=1 uv run pytest tests/scripts/test_postgres_transition.py -q

sqlx-check:
    uv run python scripts/postgres_check.py

sqlx-prepare:
    uv run python scripts/postgres_check.py --prepare
