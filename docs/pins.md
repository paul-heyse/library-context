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

## Analyzers (open — first decision of increment 1)

| Component | Candidates | Notes |
|---|---|---|
| ruff crates | Initial_plan audited `660350be2648e60e0c241e24a6ed05a38b4098fa`; `pyrefly-ruff` skill indexes ruff 0.16.7 / crates 0.0.13 | library crates are a separate 0.0.x line |
| pyrefly | Initial_plan audited `9733bdcfdf05355f816f8d8f919e01bd034efdb7` (1.4.0-dev.1); skill and dev env use 1.3.1 | crates unpublished; git dependency |

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
