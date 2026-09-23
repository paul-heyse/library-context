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
| markdown (markdown-rs) | =1.0.0, default features (none): the docs family's MDX parser (CPG slice C5) | 2026-09-23 | the crate's `Cargo.toml` read from the registry: MIT, one required dependency (`unicode-id` 0.3). Probe P5: `ParseOptions::mdx()` plus frontmatter parsed 144 of FastMCP 4.0.5's 148 guide pages; the 4 `snippets/*.mdx` React components are JS it cannot read (their coverage says so). Offsets are **bytes** (every inline-code and code-block value reproduced by slicing at them, 117 non-ASCII files). No skill covers it; Context7 `/wooorm/markdown-rs` was the lead. `just deps` and cargo-deny in `just test-all` |

## Analyzers (ADR-0012, accepted)

Both analyzers are workspace dependencies since increment 1, slice 1 (`Cargo.toml`, `Cargo.lock`).
These rows record what spike `spike/pyrefly-inproc` (`d00bab5`) read, built and checked.

| Component | Pin | Verified | How |
|---|---|---|---|
| pyrefly (library) | git `github.com/paul-heyse/pyrefly` rev `6a93da3460f9279ba32ff4ebec07e1cbdf2eb978`, in-process. That is tag 1.3.1 (`3e3177d0f4755b56c2d5a710d830eed89b14c2e3`) plus `third_party/pyrefly-1.3.1.patch` (sha256 `5782fe3e4fe62790e9039783a8be0863c9293f98f0fd73146099189909db4358`; 7 files, +30/−12: visibility, a `write_files` switch, a `pysa_reporter()` borrow; C4 adds `ClassField::dataclass_flags_of` public and a `Transaction::get_wildcard` accessor composing the existing `lookup_export`/`wildcard` queries, still no logic change and under ADR-0012's ~60-line trigger). Published 2026-09-23 as branch `lctx/1.3.1-r2`; the first revision `b9f28575` stays on branch `lctx/1.3.1`, so older commits of this repository still build | 2026-09-23 | `git ls-remote https://github.com/paul-heyse/pyrefly.git refs/heads/lctx/1.3.1-r2` gives `6a93da34` (and `refs/heads/lctx/1.3.1` still gives `b9f28575`). `git format-patch -1 --stdout` on the clone the branch was pushed from gives the patch sha256. Its parent is the 1.3.1 tag (`check_pyrefly_fork.py`) |
| pyrefly (CLI) | 1.3.1 (uv dev group). **Parity-test oracle only** | 2026-09-22 | `uv run pyrefly --version`; spike S4 |
| ruff library crates | `=0.0.11`, in-process: the line pyrefly 1.3.1 requires (`ruff_python_ast`, `ruff_python_parser`, `ruff_source_file`, `ruff_text_size`, `ruff_notebook`, `ruff_annotate_snippets`) | 2026-09-22 | pyrefly 1.3.1 `pyrefly/Cargo.toml` L69–L74 (`"0.0.11"`, which is exact on the 0.0.x line); the spike `Cargo.lock` resolves only 0.0.11. The pyrefly-ruff skill indexes **0.0.13**. Its contracts transfer except for the known deltas: `TokenKind::is_dot`/`is_lbrace`, `TokenIterWithContext::new`, `parenthesized_range` on unclosed calls, `case +1` handling, lexer accessors |
| blake3 | `=1.8.6` (pyrefly's exact pin; DataFusion's `"1.8"` accepts it) | 2026-09-22 | pyrefly 1.3.1 `pyrefly/Cargo.toml` L40; spike `Cargo.lock` |
| git sources | `github.com/paul-heyse/pyrefly`; `github.com/yangdanny97/lsp-types` rev `395d6bfcd6c3696a64cfe9cd93b86f981fb85112` (used by `pyrefly_python` and `pyrefly_util`) | 2026-09-22 | spike `deny.toml` `allow-git`. Licenses pyrefly adds: 0BSD, ISC, Unicode-DFS-2016, BSL-1.0 |
| research-input revisions | ruff `660350be…`, pyrefly `9733bdcf…` (1.4.0-dev.1) | — | **not adopted**: untagged; revisit when 1.4.x is on PyPI |

## Analyzed libraries (ADR-0013; the pilot per ADR-0004)

Each analyzed library pins itself in `libraries/<name>/` (`pyproject.toml`, `.python-version`,
`uv.lock`); these rows only summarize. `uv lock --project libraries/<name> --check` confirms a lock.

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (analyzed, the pilot) | `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`; release `fastmcp`, `fastmcp-slim`, `fastmcp-tasks` 4.0.5; `mcp`/`mcp-types` 2.2.0; 109 locked packages; Python 3.14.7. Docs/examples/tests: `PrefectHQ/fastmcp@v4.0.5` (fetched in increment 3) | 2026-09-22 | `libraries/fastmcp/uv.lock` (`uv lock`, uv 0.12.18); `lctx acquire fastmcp` then Stage A verified 275 release files against their `RECORD`s; `just pilot` published a snapshot |
| gold reference | fastmcp skill at **4.0.5** (same install line; release 4.0.5; mcp 2.2.0): capability families (22), evaluation only. 49 reviewed claims validate against 4.0.5 (12 re-hashed, 1 re-reviewed, `fm.inputs.r02.1` rewritten: with no active Context or on a lax server the strictness helper returns `None`, so per-parameter `strict=True` holds); 37/37 runtime cases pass | 2026-09-22 | re-pinned through its own `MAINTENANCE.md` pipeline on a copy (`replay_artifacts.py --apply`, `offline_environments.py`, `acquire.py`, `capture_sources.py`, `full_contracts.py`, `refresh_capture.py`, `build.py`, `verify.py` 12/12 with byte-identical rebuild, `qualify.py`), then swapped in; the 4.0.3 original is archived at `~/skill-work/fastmcp-4.0.3-original`. `just gold` (`scripts/check_gold.py`): ok |

## Analytics crates (ADR-0011, proposed; planned, not yet dependencies)

| Component | Pin | Verified | How |
|---|---|---|---|
| leiden-rs | `=0.8.1`, `default-features = false, features = ["petgraph"]` | 2026-09-22 (source read) | crates.io source: seed option, CPM, multiplex, rayon optional, petgraph `^0.8` |
| rand | pin whatever leiden-rs 0.8.1 resolves (0.9.x) exactly | — | increment 2 |

## Serving and embeddings (ADR-0010, accepted)

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (served) | 4.0.x (4.0.5 installed in the project environment; independent of the analyzed pin) | 2026-09-22 | the 4.0.3 → 4.0.5 source diff changes behaviour, not only logging (DESIGN §1.4) |
| vLLM | 0.30.0, **separate service environment**, `--runner pooling` | 2026-09-22 | spike E1 ran `vllm serve Qwen/Qwen3-Embedding-8B --revision … --runner pooling --max-model-len 8192` on the RTX 5090 (from the project `.venv`; it moves to its own environment in increment 1) |
| Qwen/Qwen3-Embedding-8B | revision `1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`; 4,096 dims; bf16 weights (15 GB), float32 output, L2-normalized (pooling `LAST` + sentence-transformers normalize). Operator choice over 4B (ADR-0010) | 2026-09-22 | HF API `sha` at that revision; `hf download … --revision`; spike E1 served it with vLLM 0.30.0 and every norm was 1 ± 1e-7 |
| pyarrow | 25.0.1 (the bundle reader in `lctx_mcp`; cp314 wheels) | 2026-09-22 | spike E3 ran the MCP round trip on it through `uv run --with pyarrow`; locked when `pyproject.toml` is restructured in increment 1 |
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
