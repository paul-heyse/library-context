# The only command surface agents need. `just` lists recipes.
# Check recipes do not edit source; tests may create their own data fixtures.
# `fmt` and `adr new|supersede|index` edit the working tree.

set shell := ["bash", "-euo", "pipefail", "-c"]

# List recipes
default:
    @just --list

# The default loop: format check, lints, optimized cached core tests, rules, ADRs, agent config
check: fmt-check lint test py-check rules-scan rules-test lint-agents
    uv run python scripts/adr.py lint

# Everything: check + fixtures + dependency policy
test-all: check fixtures-check deps gold

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
    cargo clippy --release --workspace --all-targets --quiet -- -D warnings
    uv run ruff check --quiet

# Core Rust tests share the release profile with the shipped binary. Cargo rebuilds only changed
# Rust inputs; repeated runs execute cached optimized test binaries. Tests own their data state.
# Snapshots never auto-accept (INSTA_UPDATE=no).
test *args:
    INSTA_UPDATE=no cargo nextest run --release --workspace --no-tests=pass {{args}}

# Python tests (scripts and lctx_mcp over the fixture generation) + pyrefly
py-check: py-fixture
    uv run pytest
    uv run pyrefly check --summary=none

# The structured evaluation's packet for one stage (ADR-0021; DESIGN §12): targets beside answers
# Search items rank on live query vectors when embed_url names a running `just embed-serve`
structured-eval generation stage embed_url="":
    uv run python scripts/structured_eval.py {{generation}} eval/behavior/fastmcp-4.0.5.toml --stage {{stage}} --out build/structured/stage{{stage}}.md {{ if embed_url != "" { "--embed-url " + embed_url } else { "" } }}

# The generation the lctx_mcp tests serve: analysis_shapes with fake vectors, into build/py-fixture
py-fixture:
    LCTX_PY_FIXTURE="$PWD/build/py-fixture" INSTA_UPDATE=no cargo nextest run --release -p cpg-core --no-fail-fast -E 'test(writes_the_python_fixture_generation)' --status-level none --final-status-level fail

# Pinned-family single-version check + cargo-deny sources/licenses + the Pyrefly fork (ADR-0046)
deps:
    uv run python scripts/check_family.py Cargo.lock
    cargo deny --log-level error check bans sources licenses
    uv run python scripts/check_pyrefly_fork.py
    # A dependency no crate uses pins nothing (H1 O2).
    cargo shear

# The gold reference and the analyzed library name one FastMCP (ADR-0046). Separate from `deps`,
# so a skill refresh in progress never reads as a dependency-family break (ADR-0002's trigger).
gold:
    uv run python scripts/check_gold.py

# Byte-compile Python fixtures (input data) so they cannot silently become syntax-error cases
fixtures-check:
    #!/usr/bin/env bash
    set -euo pipefail
    mapfile -t files < <(find fixtures/python -name '*.py' -not -path '*/_invalid/*')
    [ ${#files[@]} -eq 0 ] && { echo "fixtures-check: not_run (no fixtures yet)"; exit 0; }
    uv run python -c 'import ast,sys; [ast.parse(open(f,"rb").read(), f) for f in sys.argv[1:]]' "${files[@]}"
    echo "fixtures-check: ${#files[@]} files parse"

# The real-library oracle (ADR-0046): acquire the FastMCP pilot from libraries/fastmcp, then
# extract, derive, validate and publish a snapshot into build/store. First run needs the network.
pilot store="build/store":
    cargo build --release -p lctx --quiet
    target/release/lctx compile fastmcp --store {{store}} --embedder fake | tee build/pilot.log
    uv run python -m lctx_mcp.smoke "$(grep '^generation ' build/pilot.log | cut -d' ' -f2)" --embedder fake

# The same compile with live vectors: needs `just embed-serve` running (else `blocked`)
pilot-live:
    cargo build --release -p lctx --quiet
    target/release/lctx compile fastmcp --store build/store --embedder vllm | tee build/pilot-live.log
    uv run python -m lctx_mcp.smoke "$(grep '^generation ' build/pilot-live.log | cut -d' ' -f2)" --embedder vllm

# The embedding service (DESIGN §11.1, ADR-0043): vLLM 0.30.0 from the locked services/vllm
# project, serving Qwen3-Embedding-8B at its pinned revision on the local GPU
embed-serve port="8000":
    uv run --project services/vllm --frozen vllm serve Qwen/Qwen3-Embedding-8B --revision 1d8ad4ca9b3dd8059ad90a75d4983776a23d44af --runner pooling --max-model-len 8192 --dtype bfloat16 --gpu-memory-utilization 0.80 --port {{port}}

# The live leg of the client conformance check (§11.1, E2): needs `just embed-serve` running
embed-conformance url="http://127.0.0.1:8000":
    LCTX_EMBED_URL={{url}} LCTX_CONFORMANCE_OUT="$PWD/build/conformance-rust.json" cargo nextest run --release -p lctx-embed -E 'test(live_conformance_vectors)' --status-level none --final-status-level fail
    uv run python scripts/embed_conformance.py build/conformance-rust.json --url {{url}}

# Gold scores of a generation (DESIGN §12; matcher 2, ADR-0043). Exits 2 when
# `vllm` was asked for and any alias degraded (`blocked`)
score generation embedder="none":
    uv run python scripts/score_gold.py {{generation}} --embedder {{embedder}} --json build/score-$(basename {{generation}})-{{embedder}}.json

# The §1.5 retrieval check over a generation (ADR-0043): exits 1 on a miss. Only
# `--embedder vllm` (with `just embed-serve` running) makes a hybrid result evidence
ranking-check generation embedder="none":
    uv run python scripts/ranking_check.py {{generation}} --embedder {{embedder}}

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
    for t in cargo rustc cargo-nextest cargo-insta cargo-deny cargo-shear uv ast-grep rg git gh clang clang++ llvm-config mold sccache; do
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
