# Pins

Every pin with how and when it was last verified. Change a pin with the
`pin-check` skill: read the primary source, change it, run `just deps` and the
tests, and add a dated row here. Design authority for the Rust family is
DESIGN §7 / ADR-0002. A row without a date is not verified.

## Rust

| Component | Pin | Verified | How |
|---|---|---|---|
| toolchain | 1.98.1 (`rust-toolchain.toml`) | 2026-09-22 | installed; family MSRVs: delta-rs 1.94.1, DataFusion 55.1 manifests ≤1.94 |
| datafusion | =55.1.0 | 2026-09-22 | `family_smoke` passed; single version in `Cargo.lock` |
| arrow-*, parquet | =59.3.0 | 2026-09-22 | same |
| object_store | =0.13.2 | 2026-09-22 | same |
| deltalake | git 58f07cd62bfbce3649a7e1c87c696288068ae184, features `datafusion`,`rustls` | 2026-09-22 | same; matches `.claude/skills/deltalake` capture profile |
| buoyant_kernel | 0.25.1, git 8ba063f8f84fec222000f66d40d70911d7c79675 (branch `buoyant/main`, pinned by `Cargo.lock` only) | 2026-09-22 | `Cargo.lock`; matches the skill's kernel pin |
| petgraph | =0.8.3 | 2026-09-22 | resolves; no code uses it yet |

## Analyzers (ADR-0006, proposed)

| Component | Pin | Verified | How |
|---|---|---|---|
| ruff library crates | `=0.0.13` (ruff 0.16.7 line; MSRV 1.96), in-process | 2026-09-22 (interface only) | pyrefly-ruff skill index at 0.0.13; crates.io index cache. Not yet a dependency |
| pyrefly | 1.3.1, tag `3e3177d0f4755b56c2d5a710d830eed89b14c2e3`, CLI subprocess | 2026-09-22 | `uv run pyrefly --version`; Pysa JSON / Glean / `coverage report` shapes observed on the skill fixture |
| research-input revisions | ruff `660350be…`, pyrefly `9733bdcf…` (1.4.0-dev.1) | — | **not adopted**: untagged; revisit when 1.4.x is on PyPI |

## Pilot subject (ADR-0004, proposed)

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (analyzed) | 4.0.3: `fastmcp` + `fastmcp-slim` + `fastmcp-tasks`; docs/examples/tests from `jlowin/fastmcp@v4.0.3` | 2026-09-22 | fastmcp skill `build/manifests/fastmcp.json` |
| gold reference | fastmcp skill capability families (22), evaluation only | 2026-09-22 | `content/capabilities/fm.*.json` with `authoring_sha256` |

## Analytics crates (ADR-0011, proposed; planned, not yet dependencies)

| Component | Pin | Verified | How |
|---|---|---|---|
| leiden-rs | `=0.8.1`, `default-features = false, features = ["petgraph"]` | 2026-09-22 (source read) | crates.io source: seed option, CPM, multiplex, rayon optional, petgraph `^0.8` |
| rand | pin whatever leiden-rs 0.8.1 resolves (0.9.x) exactly | — | increment 2 |

## Serving and embeddings (ADR-0010, proposed)

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (served) | 4.0.x (4.0.5 installed) | 2026-09-22 | skill 4.0.3 vs installed 4.0.5 diff: logging only |
| vLLM | 0.30.0, **separate service environment**, `--runner pooling` | 2026-09-22 (source read) | installed source in `.venv`; to move out of project deps in increment 1 |
| Qwen/Qwen3-Embedding-4B | 2,560 dims, float32, L2-normalized; model revision to pin | — | model card + `config.json`/`modules.json` read; revision hash pinned at the increment-1 spike |
| pyarrow | pin at increment 1 (the bundle reader in `lctx_mcp`; 25.0.1 has cp314 wheels) | — | to be locked when `pyproject.toml` is restructured |
| LanceDB | 0.39.0 (Python) — **deferred** behind a size trigger | 2026-09-22 (source read at tag) | hybrid/FTS/RRF chain; bundles Arrow 58 / DataFusion 54 |

## Dev tools

| Tool | Version | Verified | How |
|---|---|---|---|
| Python | 3.14.7 (`.python-version`, uv default) | 2026-09-22 | `uv run python --version` |
| ruff | 0.16.7 (uv dev group) | 2026-09-22 | `uv run ruff --version` |
| pyrefly | 1.3.1 (uv dev group) | 2026-09-22 | `uv run pyrefly --version` |
| pytest | 9.1.1 (`uv.lock`) | 2026-09-22 | `uv sync` |
| cargo-nextest | 0.9.144 | 2026-09-22 | `just doctor` |
| cargo-insta | 1.48.0 | 2026-09-22 | `just doctor` |
| cargo-deny | 0.20.2 (does not see dev-only duplicates; see ADR-0002) | 2026-09-22 | tested with a synthetic duplicate |
| ast-grep | 0.45.3 | 2026-09-22 | `just doctor` |
| just | 1.58.0 | 2026-09-22 | `just --version` |
