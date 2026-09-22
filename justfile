# The only command surface agents need. `just` lists recipes.
# Check recipes never mutate; `fmt` and `adr new|supersede|index` do.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List recipes
default:
    @just --list

# The default loop: format check, lints, core tests, rules, ADRs, agent config
check: fmt-check lint test py-check rules-scan rules-test lint-agents
    uv run python scripts/adr.py lint

# Everything: check + fixtures + dependency policy
test-all: check fixtures-check deps

# rustfmt + ruff format check (no changes)
fmt-check:
    cargo fmt --all --check
    uv run ruff format --check

# Format everything (mutating)
fmt:
    cargo fmt --all
    uv run ruff format
    uv run ruff check --fix --quiet

# clippy (deny warnings) + ruff lint
lint:
    cargo clippy --workspace --all-targets --quiet -- -D warnings
    uv run ruff check --quiet

# Core Rust tests; snapshots never auto-accept (INSTA_UPDATE=no)
test *args:
    INSTA_UPDATE=no cargo nextest run --workspace --no-tests=pass {{args}}

# Python script tests + pyrefly
py-check:
    uv run pytest
    uv run pyrefly check --summary=none

# Pinned-family single-version check + cargo-deny sources/licenses + the Pyrefly fork (ADR-0012)
deps:
    uv run python scripts/check_family.py Cargo.lock
    cargo deny --log-level error check bans sources licenses
    uv run python scripts/check_pyrefly_fork.py

# Byte-compile Python fixtures (input data) so they cannot silently become syntax-error cases
fixtures-check:
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t files < <(find fixtures/python -name '*.py' -not -path '*/_invalid/*')
    [ ${#files[@]} -eq 0 ] && { echo "fixtures-check: not_run (no fixtures yet)"; exit 0; }
    uv run python -c 'import ast,sys; [ast.parse(open(f,"rb").read(), f) for f in sys.argv[1:]]' "${files[@]}"
    echo "fixtures-check: ${#files[@]} files parse"

# ast-grep scan over the tree (rules/ grows from design-review findings)
rules-scan:
    @if ls rules/*.yml >/dev/null 2>&1; then ast-grep scan; else echo "rules-scan: not_run (no rules yet)"; fi

# ast-grep rule fixtures
rules-test:
    @if ls rules/*.yml >/dev/null 2>&1; then ast-grep test --skip-snapshot-tests; else echo "rules-test: not_run (no rules yet)"; fi

# ADR tooling: new <slug> [--title T] | supersede <ADR-NNNN> <slug> | index | lint | revisit
[positional-arguments]
adr *args:
    @uv run python scripts/adr.py "$@"

# Claude/Codex parity and dead-reference check for agent instructions and skills
lint-agents:
    uv run python scripts/check_agents.py

# Tool presence and versions (compare with docs/pins.md)
doctor:
    #!/usr/bin/env bash
    for t in cargo rustc cargo-nextest cargo-insta cargo-deny uv ast-grep rg git gh; do
      printf '%-14s ' "$t"; command -v "$t" >/dev/null && "$t" --version 2>/dev/null | head -1 || echo MISSING
    done
    printf '%-14s ' ruff; uv run ruff --version
    printf '%-14s ' pyrefly; uv run pyrefly --version
    printf '%-14s ' python; uv run python --version
