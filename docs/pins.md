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
| arrow-*, parquet | =59.3.0; `parquet` is a direct dependency since H1 P6 (`default-features = false, features = ["zstd"]`: the codec was already compiled) | 2026-09-23 | same; `data_files_are_zstd` |
| object_store | 0.13.2, held by `Cargo.lock` (no crate depends on it directly, so a workspace pin would pin nothing; H1 O2) and kept single by `check_family.py` | 2026-09-23 | `Cargo.lock`; `cargo shear` in `just deps` |
| deltalake | git 58f07cd62bfbce3649a7e1c87c696288068ae184, features `datafusion`,`rustls` | 2026-09-22 | same; matches `.claude/skills/deltalake` capture profile |
| buoyant_kernel | 0.25.1, git 8ba063f8f84fec222000f66d40d70911d7c79675 (branch `buoyant/main`, pinned by `Cargo.lock` only) | 2026-09-22 | `Cargo.lock`; matches the skill's kernel pin |
| petgraph | =0.8.3, default features (no `rayon`, `serde-1`): the §5 adapter and Pass A in `lctx-analytics` (increment 1 slice 1.4) | 2026-09-23 | resolves; `lctx-analytics` tests (shuffled arcs give identical adjacency; parallel arcs and isolates kept) |
| reqwest | =0.12.28, `default-features = false`: the compile-time embedding client over plain HTTP to the local vLLM service (`lctx-embed`, DESIGN §11.1) | 2026-09-23 | already in `Cargo.lock` at this version (object_store), no feature added and no package added; MIT OR Apache-2.0; `lctx-embed` tests (stub service) |
| sha2 | =0.10.9 in `cpg-core`: the embedding spec hash and input hashes (SHA-256, so Python recomputes them) | 2026-09-23 | already a workspace pin (Stage A's `RECORD` hashes) |
| unicode-segmentation | =1.13.3: Stage F's lead sentences, UAX #29 `split_sentence_bound_indices` with byte offsets (DESIGN §10.3, slice 1.5) | 2026-09-23 | already in `Cargo.lock` at this version (arrow-cast → comfy-table), so no version is added; MIT OR Apache-2.0 |
| fixedbitset | =0.5.7: subsystem masks for `NodeFiltered`, later FCA contexts (§9.6) | 2026-09-23 | already in `Cargo.lock` as petgraph's dependency at this version, so no version is added; MIT OR Apache-2.0 |
| markdown (markdown-rs) | =1.0.0, default features (none): the docs family's MDX parser (CPG slice C5) | 2026-09-23 | the crate's `Cargo.toml` read from the registry: MIT, one required dependency (`unicode-id` 0.3). Probe P5: `ParseOptions::mdx()` plus frontmatter parsed 144 of FastMCP 4.0.5's 148 guide pages; the 4 `snippets/*.mdx` React components are JS it cannot read (their coverage says so). Offsets are **bytes** (every inline-code and code-block value reproduced by slicing at them, 117 non-ASCII files). No skill covers it; Context7 `/wooorm/markdown-rs` was the lead. `just deps` and cargo-deny in `just test-all` |

## Utility crates (H1, the library-leverage review)

Each was already in `Cargo.lock` at exactly this version (a dependency of Pyrefly, delta-rs or
DataFusion) before it became a direct dependency, so none adds a version; `just deps` passes.

| Component | Pin | Verified | How |
|---|---|---|---|
| globset | =0.4.20: corpus globs (`literal_separator`) | 2026-09-23 | `Cargo.lock` before and after; Unlicense OR MIT; the glob tests and the pilot's identical selection |
| walkdir | =2.5.0: the tree, source-tree and environment walks (`follow_links(false)`) | 2026-09-23 | same; Unlicense OR MIT |
| serde | =1.0.229, `derive`: typed `pyproject.toml` (`[tool.lctx]`, `deny_unknown_fields`) and `uv.lock` | 2026-09-23 | same; MIT OR Apache-2.0; `toml`'s serde feature was already on |
| csv | =1.4.0: `RECORD` read as CSV (PEP 376 quotes a path holding `,` or `"`) | 2026-09-23 | same; Unlicense OR MIT; `a_quoted_record_path_is_verified` |
| clap | =4.6.7, `derive`: the `lctx` and `lctx-extract` command lines (no `env` reads) | 2026-09-23 | same; MIT OR Apache-2.0; the CLI parse tests and `compile_honours_reinstall` |
| tikv-jemallocator | =0.7.0 (jemalloc 5.3.1): the binaries' global allocator, Linux/macOS (ADR-0016) | 2026-09-23 | same (Pyrefly's Linux/macOS dependency, already compiled; its CLI uses it); MIT/Apache-2.0, C source BSD-2-Clause; the allocator spike's 16 runs and H1's pilots |
| tracing-subscriber | =0.3.23, `env-filter` (defaults `fmt`, `tracing-log` already on): the binaries' stderr log subscriber, `LCTX_LOG` (H1 O1) | 2026-09-23 | same; MIT; `LCTX_LOG=debug lctx query …` prints delta-kernel and DataFusion records |
| fs-err | =3.3.1: filesystem calls whose errors name their path (`cpg-extract`, `cpg-core`, `lctx`; H1 O3) | 2026-09-23 | same; MIT OR Apache-2.0; `an_io_error_names_its_path` |
| anyhow | =1.0.104: `lctx`'s error type, printed as the whole chain (H1 O4) | 2026-09-23 | same; MIT OR Apache-2.0 |

## Supporting crates (H1 review F8)

Exact pins at the versions `Cargo.lock` already held, so none moved; `just deps` fails on a
`[workspace.dependencies]` entry without an exact pin or a row here.

| Component | Pin | Verified | How |
|---|---|---|---|
| tokio | =1.53.1 (`macros`, `rt-multi-thread`) | 2026-09-23 | `Cargo.lock`; `just deps` |
| base64 | =0.22.1: `RECORD` hashes (Stage A, ADR-0013) | 2026-09-23 | same (already in the graph at this version) |
| getrandom | =0.3.4: each attempt's `snapshot_id` (§3.4.1) | 2026-09-23 | same |
| futures | =0.3.34 (the validation stream, H1 P2) | 2026-09-23 | same |
| serde_json | =1.0.151 | 2026-09-23 | same |
| thiserror | =2.0.20 | 2026-09-23 | same |
| url | =2.5.8 | 2026-09-23 | same |
| insta | =1.48.0 (dev) | 2026-09-23 | same |
| proptest | =1.11.0 (dev) | 2026-09-23 | same |
| tempfile | =3.27.0 (dev) | 2026-09-23 | same |

## Analyzers (ADR-0012, accepted)

Both analyzers are workspace dependencies since increment 1, slice 1 (`Cargo.toml`, `Cargo.lock`).
These rows record what spike `spike/pyrefly-inproc` (`d00bab5`) read, built and checked.

| Component | Pin | Verified | How |
|---|---|---|---|
| pyrefly (library): `pyrefly`, `pyrefly_build`, `pyrefly_config`, `pyrefly_python`, `pyrefly_types`, `pyrefly_util` | git `github.com/paul-heyse/pyrefly` rev `a07b7baead9e0c7b496346d879b88e2fff9cbda7`, in-process. That is tag 1.3.1 (`3e3177d0f4755b56c2d5a710d830eed89b14c2e3`) plus `third_party/pyrefly-1.3.1.patch` (sha256 `fc18dc4a884a8593220370ba053968fd10de65c020ef257931f97b91426fdb73`; 8 files, 54 changed lines (42+/12−): visibility, a `write_files` switch, a `pysa_reporter()` borrow; C4's `ClassField::dataclass_flags_of` and `Transaction::get_wildcard`; H1 D6's `Answers::get_annotation`, composing `key_to_idx_hashed_opt` and `get_idx`: still no logic change and under ADR-0012's ~60-line trigger). Published 2026-09-23 as branch `lctx/1.3.1-r3`; `lctx/1.3.1-r2` (`6a93da34`) and `lctx/1.3.1` (`b9f28575`) stay, so older commits of this repository still build | 2026-09-23 | `git ls-remote https://github.com/paul-heyse/pyrefly.git 'refs/heads/lctx/*'` gives `a07b7bae` for `r3` (and `6a93da34`, `b9f28575` unchanged). `git format-patch -1 --stdout` on the clone the branch was pushed from gives the patch sha256. Its parent is the 1.3.1 tag (`check_pyrefly_fork.py`). The pilot's outputs were fingerprint-identical on the patched fork before the push |
| pyrefly (CLI) | 1.3.1 (uv dev group). **Parity-test oracle only** | 2026-09-22 | `uv run pyrefly --version`; spike S4 |
| ruff library crates | `=0.0.11`, in-process: the line pyrefly 1.3.1 requires (`ruff_python_ast`, `ruff_python_parser`, `ruff_source_file`, `ruff_text_size`, `ruff_notebook`, `ruff_annotate_snippets`; from slice 2.2's review, `ruff_python_stdlib` for the usage patterns' builtins, whose bitflags and unicode-ident were already locked) | 2026-09-22 (stdlib 2026-09-23, `Cargo.lock`) | pyrefly 1.3.1 `pyrefly/Cargo.toml` L69–L74 (`"0.0.11"`, which is exact on the 0.0.x line); the spike `Cargo.lock` resolves only 0.0.11. The pyrefly-ruff skill indexes **0.0.13**. Its contracts transfer except for the known deltas: `TokenKind::is_dot`/`is_lbrace`, `TokenIterWithContext::new`, `parenthesized_range` on unclosed calls, `case +1` handling, lexer accessors |
| blake3 | `=1.8.6` (pyrefly's exact pin; DataFusion's `"1.8"` accepts it) | 2026-09-22 | pyrefly 1.3.1 `pyrefly/Cargo.toml` L40; spike `Cargo.lock` |
| git sources | `github.com/paul-heyse/pyrefly`; `github.com/yangdanny97/lsp-types` rev `395d6bfcd6c3696a64cfe9cd93b86f981fb85112` (used by `pyrefly_python` and `pyrefly_util`) | 2026-09-22 | spike `deny.toml` `allow-git`. Licenses pyrefly adds: 0BSD, ISC, Unicode-DFS-2016, BSL-1.0 |
| the flow provider (ADR-0022 §The flow provider, ADR-0012 amendment): `ty_python_core`, `ty_module_resolver`, `ty_vendored`, `ruff_db`, and ruff's `ruff_python_ast`, `ruff_python_parser` and `ruff_text_size` (workspace keys `ruff_python_ast_ty`, `ruff_python_parser_ty`, `ruff_text_size_ty`; the parser for the rename's tokens) | `=0.0.14`, in-process, **only in `cpg-flow`** (a declared extra family: `scripts/check_family.py` `EXTRA_FAMILIES`). `salsa` `=0.28.2`, with `salsa-macros` and `salsa-macro-rules` held at 0.28.2 in `Cargo.lock` (`cargo update -p <crate> --precise 0.28.2`): 0.28.3 and 0.28.4 break ruff 0.0.14 | 2026-09-24 | Each crate's `Cargo.toml` in `~/.cargo/registry/src` reads `version = "0.0.14"`, and salsa's `0.28.2`; `just deps` (the family check with the extra family's scope and pins). The Stage 2.1 spike built the same set; `cargo nextest run -p cpg-flow` 14/14 (`flow_shapes` known answers). No skill indexes ty; the `pyrefly-ruff` skill's 0.0.13 ruff contracts are the nearest reference. `ruff_db` reads `TY_MAX_PARALLELISM` and `RAYON_NUM_THREADS`: parallelism only, and the index is built per file on the driver thread, so they are output-neutral (the Stage 2 review's O2) |
| research-input revisions | ruff `660350be…`, pyrefly `9733bdcf…` (1.4.0-dev.1) | — | **not adopted**: untagged; revisit when 1.4.x is on PyPI |

## Analyzed libraries (ADR-0013; the pilot per ADR-0004)

Each analyzed library pins itself in `libraries/<name>/` (`pyproject.toml`, `.python-version`,
`uv.lock`); these rows only summarize. `uv lock --project libraries/<name> --check` confirms a lock.

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (analyzed, the pilot) | `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`; release `fastmcp`, `fastmcp-slim`, `fastmcp-tasks` 4.0.5; `mcp`/`mcp-types` 2.2.0; 109 locked packages; Python 3.14.7. Docs/examples/tests: `PrefectHQ/fastmcp@v4.0.5` (fetched in increment 3) | 2026-09-22 | `libraries/fastmcp/uv.lock` (`uv lock`, uv 0.12.18); `lctx acquire fastmcp` then Stage A verified 275 release files against their `RECORD`s; `just pilot` published a snapshot |
| gold reference | fastmcp skill at **4.0.5** (same install line; release 4.0.5; mcp 2.2.0): capability families (22), evaluation only. 49 reviewed claims validate against 4.0.5 (12 re-hashed, 1 re-reviewed, `fm.inputs.r02.1` rewritten: with no active Context or on a lax server the strictness helper returns `None`, so per-parameter `strict=True` holds); 37/37 runtime cases pass | 2026-09-22 | re-pinned through its own `MAINTENANCE.md` pipeline on a copy (`replay_artifacts.py --apply`, `offline_environments.py`, `acquire.py`, `capture_sources.py`, `full_contracts.py`, `refresh_capture.py`, `build.py`, `verify.py` 12/12 with byte-identical rebuild, `qualify.py`), then swapped in; the 4.0.3 original is archived at `~/skill-work/fastmcp-4.0.3-original`. `just gold` (`scripts/check_gold.py`): ok |

## Analytics crates (ADR-0011, accepted; leiden-rs from slice 2.3, fcars from 2.5)

| Component | Pin | Verified | How |
|---|---|---|---|
| leiden-rs | `=0.8.1`, `default-features = false` and no features (ADR-0011 as amended: built from the dense index, not the `petgraph` adapter; sequential, no rayon) | 2026-09-23 (slice 2.3) | `Cargo.lock` gains only `leiden-rs 0.8.1` (its rand 0.9, rustc-hash 2 and thiserror 2 were already locked); `lctx-analytics` `build.rs` records it in every invocation's `library_versions`; the LFR and shuffled-input fixtures in `communities::tests` |
| fcars, bitvec | `=0.2.2` and `=1.0.1`, dev-dependencies of `lctx-analytics` only: fcars is the oracle for our FCA's concept sets (ADR-0011), bitvec builds its relation | 2026-09-23 (slice 2.5) | MIT; the lock gains bitvec, funty, radium, tap and wyz (dev only; rayon was already locked); `fcars_agrees_on_the_concepts` compares concept sets on random contexts |
| rand, rand_chacha, rand_core | the 0.9 line leiden-rs resolves: rand 0.9.5, rand_chacha 0.9.0, rand_core 0.9.5 (the lock also holds rand 0.8 and 0.10 for other crates) | 2026-09-23 (slice 2.3) | `lctx-analytics` `build.rs` asserts each is locked exactly once on the `0.9.` line and records it; rand does not promise sequences across versions (ADR-0011) |

## Serving and embeddings (ADR-0010, accepted)

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (served) | `==4.0.5` in `python/lctx_mcp` (the workspace `uv.lock`; mcp 2.2.0, pydantic 2.13.5), independent of the analyzed pin | 2026-09-23 | `uv lock`; the lctx_mcp tests negotiate `2026-07-28` (auto) and `2025-11-25` (legacy) |
| vLLM | 0.30.0 in its own locked uv project `services/vllm` (`uv.lock`: torch and CUDA pinned with it; ADR-0010 amendment), served by `just embed-serve`: `--runner pooling --max-model-len 8192 --dtype bfloat16 --gpu-memory-utilization 0.80` | 2026-09-23 | `uv lock --project services/vllm` resolved 204 packages; spike E1 (2026-09-22) ran the same service from the project `.venv` |
| Qwen/Qwen3-Embedding-8B | revision `1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`; 4,096 dims; bf16 weights (15 GB), float32 output, L2-normalized (pooling `LAST` + sentence-transformers normalize). Operator choice over 4B (ADR-0010) | 2026-09-22 | HF API `sha` at that revision; `hf download … --revision`; spike E1 served it with vLLM 0.30.0 and every norm was 1 ± 1e-7 |
| pyarrow | `==25.0.1` (the bundle reader in `lctx_mcp`; cp314 wheels) | 2026-09-23 | `uv lock` (slice 1.8); the serving-digest tests read the generation's files with it |
| numpy | `==2.4.6` (vectors, exact cosine; bm25s's only dependency) | 2026-09-23 | `uv lock`; `uv run python -c "import numpy"` |
| httpx2 | `==2.13.1` (the query embedder; bytes sent as `content=`, never `json=`). pydantic's maintained continuation of httpx (operator, 2026-09-24); FastMCP 4.0.5 already depends on it, so the switch removes `httpx` 0.28.1 from the server's lock | 2026-09-24 | PyPI JSON read 2026-09-24 (2.13.1, 2026-09-23; `import httpx2`); `uv lock`; `test_the_http_client_sends_the_exact_bytes_and_reports_a_down_service` |
| bm25s | `==0.3.11`, numpy backend; it depends on numpy only, so no scipy enters the environment | 2026-09-23 | `uv.lock` (`dependencies = [numpy]`); `test_bm25_scores_are_the_lucene_formula` |
| uv_build | `>=0.12,<0.13` (the `lctx-mcp` build backend, matching uv 0.12.18) | 2026-09-23 | `uv sync` built the member |
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
