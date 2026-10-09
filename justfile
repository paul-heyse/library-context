# The only command surface agents need. `just` lists recipes.
# Check recipes do not edit source; tests may create their own data.
# `fmt`, `skills-sync`, `build-features` and `adr new|supersede|index` edit the
# working tree. Agents run scoped contract families and applicable leaves. Two bundles, each
# keeping going after a failure: `turn-end` at the end of a turn that changed files, and `ready`
# after an environment change.

set shell := ["python3", "scripts/build_environment.py", "--", "bash", "-euo", "pipefail", "-c"]

# List recipes
default:
    @just --list

turn_end_steps := "adr-index build-features fmt"
ready_steps := "skills-sync _sync-native doctor-check _ready-report"

# End of a turn that changed files: regenerate the ADR index and Hakari crate, and format.
# `just turn-end --paths P…` or `--staged` scopes this to the turn's own paths when the tree holds
# another agent's uncommitted work (ADR-0134); skipped steps are reported for a later whole-tree run.
[positional-arguments]
turn-end *args:
    @if [ "$#" -eq 0 ]; then just _bundle turn-end "{{ turn_end_steps }}"; else uv run --no-project --offline --no-python-downloads python scripts/maintenance.py turn-end "$@"; fi

# Run after a dependency, toolchain or skill-selection change, or an environment-shaped failure,
# in any checkout. It selects this checkout's own `.venv` (replacing, and reporting, an inherited
# UV_PROJECT_ENVIRONMENT or VIRTUAL_ENV) and reports the interpreter, extension import origin,
# build dir and locks.
# Select this checkout's env, skill links, `sync native` once, doctor check, then the report
ready:
    #!/usr/bin/env bash
    set -uo pipefail
    selection=$(uv run --no-project --offline --no-python-downloads python scripts/workspace_env.py select) || exit 1
    eval "$selection"
    just _bundle ready "{{ ready_steps }}"

[private]
_ready-report:
    @uv run --no-project --offline --no-python-downloads python scripts/workspace_env.py report

# Routes: `tools` (dev tools, never builds the extension), `native` (full sync with the native
# key; a no-op while current), `vllm` (the separately locked service). Exclusive ownership; waits
# for and names live managed holders.
# Prepare one environment route: `just sync tools|native|vllm`
sync route:
    @uv run --no-project --offline --no-python-downloads python scripts/workspace_env.py sync {{ route }}

[private]
_sync-native: (sync "native")

# `--explain` (the default) reports Cargo target/build dirs, interpreters, the selected uv env and
# uv settings without fingerprinting; `-- cmd` launches with the normalized environment.
# Normalized environment: `just env [--explain] [-- cmd…]`
[positional-arguments]
env *args:
    @python3 scripts/build_environment.py {{ if args == "" { "--explain" } else { "" } }} "$@"

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

# `--select FAMILY[:BOUNDARY]` (repeatable) with `--nextest-args "…"`/`--pytest-args "…"` scoped
# to the preceding selection; `--print` (static), `--list` (builds), `--rerun RUN`, `--live`.
# Execution always runs under a run handle (`just runs`), with summary.json in its directory.
# One resolved command plan over boundary definitions (D4); `just verify --help` lists them
[positional-arguments]
verify *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py "$@"

# Opt-in compile attribution: owned recordings, live status and retained post-build reports.
[positional-arguments]
compile-profile *args:
    @uv run --no-project --offline --no-python-downloads python scripts/compile_profile.py "$@"

# Exact, format-compatible analysis tools; no product dependency resolution or native sync.
compile-profile-tools command="check":
    @uv run --no-project --offline --no-python-downloads python scripts/compile_profile_tools.py {{ command }}

# ARGS reach the boundary's primary tool (nextest, or pytest for serving:mcp/oracles/tooling:python).
# Family shortcut: `just verify-<family> [--command BOUNDARY] [-- ARGS]`
[positional-arguments]
verify-model *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py model "$@"

[positional-arguments]
verify-compiler *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py compiler "$@"

[positional-arguments]
verify-analytics *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py analytics "$@"

[positional-arguments]
verify-providers *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py providers "$@"

[positional-arguments]
verify-store *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py store "$@"

[positional-arguments]
verify-serving *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py serving "$@"

[positional-arguments]
verify-oracles *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py oracles "$@"

[positional-arguments]
verify-tooling *args:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py tooling "$@"

# Target qualification: every boundary and leaf, keep-going; observes readiness, never prepares
# or reuses retained content.
qualify:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py qualify

# Disposable native SurrealDB fixtures (D2): `-- CMD` runs CMD with LCTX_SURREAL_TEST_CONFIG and
# LCTX_COMPILER_RUNTIME_CONFIG on a run-owned server; `--keep [-- CMD]` | `--attach ID -- CMD` |
# `--stop ID` | `--restart ID` | `--list [--json]` | `--sweep`. Sweeps dead owners at start
# (`--no-sweep`); `--memory`/LCTX_FIXTURE_MEMORY sets the cap (default 16G). Exit 75 is blocked.
[positional-arguments]
fixture *args:
    @uv run --no-project --offline --no-python-downloads python scripts/surrealdb_fixture.py "$@"

# Explicit complete fixture registration over native persisted producers.
fixture-corpus:
    @uv run --no-project --offline --no-python-downloads python scripts/verify.py --select compiler:producer --nextest-args "--test fixture_corpus"

# Run any command under a handle in build/runs/<id>/ (D4): `just run [--background] [--label L] -- <cmd…>`.
# Prefer the runtime's own background/wait/cancel first; the handle crosses tool calls and sessions.
[positional-arguments]
run *args:
    @uv run --no-project --offline --no-python-downloads python scripts/runs.py run "$@"

# Run handles: list | status ID | logs ID [--follow] | cancel ID | retain ID | prune [--keep N] [--older-than D]
[positional-arguments]
runs *args:
    @uv run --no-project --offline --no-python-downloads python scripts/runs.py "$@"

# Format everything (mutating), or only named paths: `just fmt [paths…]` (`--staged` for staged
# paths). Named Rust files are formatted without their child modules (ADR-0134).
[positional-arguments]
fmt *paths:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ "$#" -gt 0 ]; then
      exec uv run --no-project --offline --no-python-downloads python scripts/maintenance.py fmt "$@"
    fi
    # Virtual-root defaults cover workspace members; --all would also rewrite vendored patches.
    cargo fmt
    uv run --no-sync ruff format
    # Apply ruff's auto-fixes; what remains is the `ruff` check's to report.
    uv run --no-sync ruff check --fix --quiet --exit-zero

# clippy, denying warnings
clippy:
    cargo clippy --release --workspace --all-targets --keep-going -- -D warnings

# Read-only lint; `just fmt` (part of `turn-end`) owns safe fixes
ruff:
    uv run --no-sync ruff check --quiet

# pyrefly type check
types:
    uv run --no-sync pyrefly check --summary=none

# ADR metadata and index
adr-lint:
    uv run --no-project --offline --no-python-downloads python scripts/adr.py lint

# Licences are never a rejection reason, so cargo-deny checks bans and sources only.
# Pinned nominal families and observational forks (ADR-0117/0118); every declared dependency exact (ADR-0132)
deps:
    uv run --no-project --offline --no-python-downloads python scripts/check_family.py Cargo.lock
    cargo deny --log-level error check bans sources
    uv run --no-project --offline --no-python-downloads python scripts/check_pyrefly_fork.py
    uv run --no-project --offline --no-python-downloads python scripts/check_ruff_fork.py
    # A dependency no crate uses pins nothing (H1 O2).
    cargo shear --exclude lctx-workspace-hack
    cargo hakari generate --diff
    cargo hakari manage-deps --dry-run

# Regenerate the executable's dependency feature union (ADR-0137; part of `turn-end`).
build-features:
    cargo hakari generate
    cargo hakari manage-deps -y

# The gold reference and the analyzed library name one FastMCP (ADR-0046). Separate from `deps`,
# so a skill refresh in progress never reads as a dependency-family break (ADR-0002's trigger).
gold:
    uv run --no-project --offline --no-python-downloads python scripts/check_gold.py

# Read-only freshness of generated outputs: clean | stale | heuristic | not_run, with each
# regenerate command (never invoked). Outputs: hakari adr-index skills rust-fmt python-fmt fmt gold insta
[positional-arguments]
fresh *args:
    @uv run --no-project --offline --no-python-downloads python scripts/freshness.py "$@"

# Byte-compile Python fixtures (input data) so they cannot silently become syntax-error cases
fixtures-check:
    #!/usr/bin/env bash
    set -euo pipefail
    eval "$(python3 scripts/build_environment.py --shell)"
    mapfile -t files < <(find fixtures/python -name '*.py' -not -path '*/_invalid/*')
    [ ${#files[@]} -eq 0 ] && { echo "fixtures-check: not_run (no fixtures yet)"; exit 0; }
    uv run --no-project --offline --no-python-downloads python -c 'import ast,sys; [ast.parse(open(f,"rb").read(), f) for f in sys.argv[1:]]' "${files[@]}"
    echo "fixtures-check: ${#files[@]} files parse"

# The embedding service (DESIGN §11.1, ADR-0080): the gpu-stack SM120 wheel from services/vllm
# project, serving Qwen3-Embedding-8B at its pinned revision on the local GPU
embed-serve port="8000":
    uv run --no-project --offline --no-python-downloads python scripts/embed_serve.py --port {{port}}

# The live leg of the client conformance check (§11.1, E2): needs `just embed-serve` running
embed-conformance url="http://127.0.0.1:8000":
    LCTX_EMBED_URL={{url}} LCTX_CONFORMANCE_OUT="$PWD/build/conformance-rust.json" cargo nextest run --release -p lctx-embed -E 'test(live_conformance_vectors)' --status-level none --final-status-level fail
    uv run --no-sync python scripts/embed_conformance.py build/conformance-rust.json --url {{url}}

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

# Tool presence and versions (compare with docs/pins.md)
doctor:
    #!/usr/bin/env bash
    eval "$(python3 scripts/build_environment.py --shell)"
    for t in cargo rustc cargo-nextest cargo-insta cargo-deny cargo-shear cargo-hakari uv ast-grep rg git gh clang clang++ llvm-config mold sccache; do
      printf '%-14s ' "$t"; command -v "$t" >/dev/null && "$t" --version 2>/dev/null | head -1 || echo MISSING
    done
    printf '%-14s ' ruff; uv run --no-sync ruff --version
    printf '%-14s ' pyrefly; uv run --no-sync pyrefly --version
    printf '%-14s ' python; uv run --no-sync python --version
    uv run --no-project --offline --no-python-downloads python scripts/docs.py doctor

# Fail when `doctor` reports a missing or failing tool (part of `ready`)
doctor-check:
    #!/usr/bin/env bash
    set -uo pipefail
    out=$(just doctor 2>&1); status=$?
    printf '%s\n' "$out"
    [ "$status" -eq 0 ] && ! grep -qE 'MISSING|FAILED|below the declared floor' <<<"$out"

# Historical build measurements: `report <dir>`. New captures use `compile-profile`.
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

# Optional checkouts (D3): worktrees serve an independent revision or conflicting mutable state;
# commands whose effects do not conflict can share one stable checkout. Creates
# ~/library-context-wt/NAME on branch wt/NAME and runs `just ready` there. `--carry PATH…` copies
# only the named paths' staged/unstaged/untracked changes. Work returns by merge or cherry-pick.
# Create a prepared worktree: `just worktree NAME [--ref R] [--carry PATH…] [--build-dir shared|own]`
[positional-arguments]
worktree *args:
    @uv run --no-project --offline --no-python-downloads python scripts/worktree.py create "$@"

# Remove a worktree and its branch; reports dirty or unintegrated work and refuses without --force
[positional-arguments]
worktree-remove *args:
    @uv run --no-project --offline --no-python-downloads python scripts/worktree.py remove "$@"
