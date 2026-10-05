# The only command surface agents need. `just` lists recipes.
# Check recipes do not edit source; tests may create their own data.
# `fmt`, `library-catalog`, `skills-sync`, `build-features` and `adr new|supersede|index` edit the
# working tree. Agents run scoped contract families and applicable leaves. Two bundles, each
# keeping going after a failure: `turn-end` at the end of a turn that changed files, and `ready`
# after an environment change.

set shell := ["python3", "scripts/build_environment.py", "--", "bash", "-euo", "pipefail", "-c"]

# List recipes
default:
    @just --list

turn_end_steps := "adr-index build-features fmt"
ready_steps := "skills-sync doctor-check"

# End of a turn that changed files: regenerate the ADR index and Hakari crate, and format
turn-end: (_bundle "turn-end" turn_end_steps)

# After a dependency, toolchain or skill-selection change, or an environment-shaped failure
ready: (_bundle "ready" ready_steps)

# Run each recipe, keep going after a failure, and list the failures
[private]
_bundle name steps:
    #!/usr/bin/env bash
    set -uo pipefail
    failed=()
    for step in {{ steps }}; do
      echo "== just $step"
      just "$step" || failed+=("$step")
    done
    if [ ${#failed[@]} -gt 0 ]; then
      echo "{{ name }}: ${#failed[@]} failed: ${failed[*]} (re-run one with just <id>)"
      exit 1
    fi
    echo "{{ name }}: all passed"

# Apply the shared library-skill selection for Codex and Claude Code
skills-sync:
    python3 scripts/library_skills.py

# Check selected library links without changing files
skills-check:
    python3 scripts/library_skills.py --check

# Explicit contract families. Boundary-specific filters use --command <boundary> -- <tool args>.
[positional-arguments]
verify-model *args:
    python3 scripts/verify.py model "$@"

[positional-arguments]
verify-compiler *args:
    python3 scripts/verify.py compiler "$@"

[positional-arguments]
verify-analytics *args:
    python3 scripts/verify.py analytics "$@"

[positional-arguments]
verify-providers *args:
    python3 scripts/verify.py providers "$@"

[positional-arguments]
verify-store *args:
    python3 scripts/verify.py store "$@"

[positional-arguments]
verify-serving *args:
    python3 scripts/verify.py serving "$@"

[positional-arguments]
verify-oracles *args:
    python3 scripts/verify.py oracles "$@"

[positional-arguments]
verify-tooling *args:
    python3 scripts/verify.py tooling "$@"

# Target qualification collects independent failures and prepares the selected union once.
qualify:
    python3 scripts/verify.py qualify

# Explicit complete fixture registration over actual store-free native producers.
fixture-corpus:
    INSTA_UPDATE=no cargo nextest run --release -p cpg-core --test fixture_corpus --no-fail-fast

# Format everything (mutating)
fmt:
    # Virtual-root defaults cover workspace members; --all would also rewrite vendored patches.
    cargo fmt
    uv run ruff format
    # Apply ruff's auto-fixes; what remains is the `ruff` check's to report.
    uv run ruff check --fix --quiet --exit-zero

# clippy, denying warnings
clippy:
    cargo clippy --release --workspace --all-targets --keep-going -- -D warnings

# Read-only lint; `just fmt` (part of `turn-end`) owns safe fixes
ruff:
    uv run --no-sync ruff check --quiet

# pyrefly type check
types:
    uv run pyrefly check --summary=none

# ADR metadata and index
adr-lint:
    uv run python scripts/adr.py lint

# Licences are never a rejection reason, so cargo-deny checks bans and sources only.
# Pinned nominal families and observational forks (ADR-0117/0118); a pins row per exact pin (ADR-0125)
deps:
    uv run python scripts/check_family.py Cargo.lock
    cargo deny --log-level error check bans sources
    uv run python scripts/check_pyrefly_fork.py
    uv run python scripts/check_ruff_fork.py
    # A dependency no crate uses pins nothing (H1 O2).
    cargo shear --exclude lctx-workspace-hack
    cargo hakari generate --diff
    cargo hakari manage-deps --dry-run

# Upgrade-specific checks join this recipe when a need emerges. `just upgrade` moves the root
# workspace (uv.lock and Cargo.lock); `just upgrade <dir> …` moves those sub-projects' locks. The
# analyzed libraries (`libraries/<name>`, moved only through `lctx library` / `--upgrade-package`,
# libraries/README.md) and `services/vllm` (custom wheel, torch/CUDA overrides) must not move
# through this recipe routinely.
# Move lockfiles to the latest versions the manifests allow, at the agent's discretion; then run the tests the move affects (ADR-0125)
upgrade *projects:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ -z "{{ projects }}" ]; then
        uv lock --upgrade
        cargo update
    else
        for p in {{ projects }}; do uv lock --upgrade --project "$p"; done
    fi

# Regenerate the executable's dependency feature union (ADR-0079; part of `turn-end`).
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

# Regenerate the ADR index (part of `turn-end`)
adr-index:
    @uv run --no-project --offline --no-python-downloads python scripts/adr.py index

# Claude/Codex parity and dead-reference check for agent instructions and skills
lint-agents:
    uv run --no-project --offline --no-python-downloads python scripts/check_agents.py

# Edits the working tree; one to two minutes (tools/lu-resolve).
# Regenerate the library catalog and the usage index behind the library-catalog MCP server
library-catalog:
    uv run python scripts/library_utilization.py --write

# Tool presence and versions (compare with docs/pins.md)
doctor:
    #!/usr/bin/env bash
    eval "$(python3 scripts/build_environment.py --shell)"
    for t in cargo rustc cargo-nextest cargo-insta cargo-deny cargo-shear cargo-hakari uv ast-grep rg git gh clang clang++ llvm-config mold sccache; do
      printf '%-14s ' "$t"; command -v "$t" >/dev/null && "$t" --version 2>/dev/null | head -1 || echo MISSING
    done
    printf '%-14s ' ruff; uv run ruff --version
    printf '%-14s ' pyrefly; uv run pyrefly --version
    printf '%-14s ' python; uv run python --version
    uv run --no-project --offline --no-python-downloads python scripts/docs.py doctor

# Fail when `doctor` reports a missing or failing tool (part of `ready`)
doctor-check:
    #!/usr/bin/env bash
    set -uo pipefail
    out=$(just doctor 2>&1); status=$?
    printf '%s\n' "$out"
    [ "$status" -eq 0 ] && ! grep -qE 'MISSING|FAILED|below the declared floor' <<<"$out"

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

