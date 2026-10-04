# Pins

Every pin with how and when it was last verified. Change a pin with the
`pin-check` skill: read the primary source; product pin changes run the tests (the end-of-turn
checks run `just deps`), and add a dated row here. Design authority for the Rust family is
DESIGN §7 / ADR-0118. A row without a date is not verified.

## Rust

| Component | Pin | Verified | How |
|---|---|---|---|
| toolchain | nightly-2026-09-29 (`rust-toolchain.toml`); manifest minimum remains 1.98.1 | 2026-09-28 | `rustc -Vv`: 1.101.0-nightly, c1070d69382b8d2f2eb65119c738a77d9e324c9e, LLVM 23.1.1; `cargo -V`: 1.101.0-nightly (3d7cf6e93). Installed rustfmt/Clippy; ADR-0079 |
| allocative | 0.3.6 with local upstream never-type backport (`third_party/allocative`) | 2026-09-28 | Published manifest/source and upstream commit `9711293c6de502d50583cafb12e4a7b764094d3a`; eight-line duplicate impl/obsolete feature removal, ADR-0079 |
| datafusion | =55.1.0 | 2026-09-29 | `family_smoke` (Arrow batches queried by DataFusion SQL) passed; single version in `Cargo.lock`. The delta-rs family and `datafusion-federation` were removed with the Delta store (plan P1.4, ADR-0118) |
| arrow-*, parquet | =59.3.0; `parquet` is a direct dependency since H1 P6 (`default-features = false, features = ["zstd"]`: the codec was already compiled) | 2026-09-23 | same; `data_files_are_zstd` |
| object_store | 0.13.2, held by `Cargo.lock` (no crate depends on it directly, so a workspace pin would pin nothing; H1 O2) and kept single by `check_family.py` | 2026-09-23 | `Cargo.lock`; `cargo shear` in `just deps` |
| petgraph | =0.8.3, default features plus `serde-1`, no `rayon`; immutable typed graph snapshots (ADR-0103) | 2026-09-30 | pinned source `graph_impl/serialization.rs` inspected; four focused snapshot controls passed (including 70,000-node multi-chunk roundtrip); full gate at Q |
| postcard | =1.1.3, no defaults, `alloc`; binary graph wrapper and actual petgraph Serde object | 2026-09-30 | downloaded registry Cargo.toml and `to_slice`/`take_from_bytes`/`experimental::serialized_size` inspected; compile and roundtrip/size/truncation/resource controls passed; full gate at Q |
| reqwest | =0.12.28, `default-features = false`: the compile-time embedding client over plain HTTP to the local vLLM service (`lctx-embed`, DESIGN §11.1) | 2026-09-23 | already in `Cargo.lock` at this version (object_store), no feature added and no package added; MIT OR Apache-2.0; `lctx-embed` tests (stub service) |
| sha2 | =0.10.9 in `cpg-core`: the embedding spec hash and input hashes (SHA-256, so Python recomputes them) | 2026-09-23 | already a workspace pin (Stage A's `RECORD` hashes) |
| unicode-segmentation | =1.13.3: Stage F's lead sentences, UAX #29 `split_sentence_bound_indices` with byte offsets (DESIGN §10.3, slice 1.5) | 2026-09-23 | already in `Cargo.lock` at this version (arrow-cast → comfy-table), so no version is added; MIT OR Apache-2.0 |
| fixedbitset | =0.5.7: subsystem masks for `NodeFiltered`, later FCA contexts (§9.6) | 2026-09-23 | already in `Cargo.lock` as petgraph's dependency at this version, so no version is added; MIT OR Apache-2.0 |
| markdown (markdown-rs) | =1.0.0, default features (none): the docs family's MDX parser (CPG slice C5) | 2026-09-23 | the crate's `Cargo.toml` read from the registry: MIT, one required dependency (`unicode-id` 0.3). Probe P5: `ParseOptions::mdx()` plus frontmatter parsed 144 of FastMCP 4.0.5's 148 guide pages; the 4 `snippets/*.mdx` React components are JS it cannot read (their coverage says so). Offsets are **bytes** (every inline-code and code-block value reproduced by slicing at them, 117 non-ASCII files). No skill covers it; Context7 `/wooorm/markdown-rs` was the lead. `just deps` and cargo-deny in `just test-all` |

## Catalog evidence parsers

PR3 adds exact pins `pep508_rs =0.9.2` (default features off), `mailparse =0.17.0`
and `rust-ini =0.21.3` (default features off). Source manifests and implementations were
**Source-reviewed, 2026-09-28** under `~/.cargo/registry/src`: Cargo.toml package/version,
`Requirement<url::Url>` parsing, raw header access and INI duplicate/case handling. The existing
`url =2.5.8` pin supplies a parser that does not expand environment variables. PEP 440 is reused
through pep508_rs. No dependency resolver is embedded. Focused parser tests passed; whole-scope
qualification is tracked in the forward plan. ADR-0076 owns this choice.

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
| tracing-subscriber | =0.3.23, `env-filter` (defaults `fmt`, `tracing-log` already on): the binaries' stderr log subscriber, `LCTX_LOG` (H1 O1) | 2026-09-23 | same; MIT; `LCTX_LOG=debug lctx flow …` prints Pyrefly and DataFusion records |
| fs-err | =3.3.1: filesystem calls whose errors name their path (`cpg-extract`, `cpg-core`, `lctx`; H1 O3) | 2026-09-23 | same; MIT OR Apache-2.0; `an_io_error_names_its_path` |
| anyhow | =1.0.104: `lctx`'s error type, printed as the whole chain (H1 O4) | 2026-09-23 | same; MIT OR Apache-2.0 |

## Supporting crates (H1 review F8)

Exact pins at the versions `Cargo.lock` already held, so none moved; `just deps` fails on a
`[workspace.dependencies]` entry without an exact pin or a row here.

| Component | Pin | Verified | How |
|---|---|---|---|
| tokio | =1.53.1 (`macros`, `rt-multi-thread`) | 2026-09-23 | `Cargo.lock`; `just deps` |
| hex | =0.4.3: cursor token encoding, unchanged lowercase hex format | 2026-10-04 | Registry `hex-0.4.3/Cargo.toml` read; existing locked version; targeted cursor controls and Q0 pending |
| base64 | =0.22.1: `RECORD` hashes (Stage A, ADR-0117) | 2026-09-23 | same (already in the graph at this version) |
| getrandom | =0.3.4: each attempt's `snapshot_id` (§3.4.1) | 2026-09-23 | same |
| futures | =0.3.34 (the validation stream, H1 P2) | 2026-09-23 | same |
| serde_json | =1.0.151 | 2026-09-23 | same |
| thiserror | =2.0.21 | 2026-10-03 | Pyrefly dev3 manifest requires ^2.0.21; targeted Cargo.lock update |
| url | =2.5.8 | 2026-09-23 | same |
| insta | =1.48.0 (dev) | 2026-09-23 | same |
| proptest | =1.11.0 (dev) | 2026-09-23 | same |
| tempfile | =3.27.0 (dev) | 2026-09-23 | same |

## Analyzers (ADR-0117)

The linked analyzer families are workspace dependencies (`Cargo.toml`, `Cargo.lock`).
These rows record the current pins and their individually scoped verification receipts; the
archived in-process Pyrefly prototype is historical evidence, not the current pin authority.

| Component | Pin | Verified | How |
|---|---|---|---|
| pyrefly (library): `pyrefly`, `pyrefly_build`, `pyrefly_bundled`, `pyrefly_config`, `pyrefly_python`, `pyrefly_types`, `pyrefly_util`, `tsp_types` | 1.4.0-dev.3, fork rev `63cda076956013cd0bd1d0d05c785f747fe6adc0`, exact parent/tag `80cec3f57364bc11d4a39a419f6894a8eabcaa00`; `third_party/pyrefly-1.4.0-dev.3.patch`, sha256 `178788fc11391c4a638306b06ba9433f15a6950dbdc184c6c5042959896e0391` | 2026-10-04 | Observation-only original overload identity and conservative selection state added (ADR-0121). Isolated pinned-toolchain compile with existing allocative patch **passed**; Three assembled native origin/replay controls and unchanged eight-case old/new types/diagnostics parity **passed**, 2026-10-04; final PostgreSQL consumers and integrated gates pending. Earlier fork receipts retain their original boundary. Embedded Ruff remains 0.0.14; old immutable refs serve historical checkouts, never runtime fallback. |
| ty IDE/reference oracle | `ty_ide`/`ty_project` unpublished 0.0.0, with transitive `ty_python_semantic` 0.0.16, at independent Ruff fork `f7bdff69e1fb94ab0ed5b340e977aac0d26e9301`; extractor development closure only | 2026-10-04 | Source manifests, release compile and `cargo test --release -p cpg-extract --test python_reference_oracle -- --nocapture` **passed**, 2026-10-04. Joined-process hermetic controls retain ResolveAliases requested-spelling filtering, ambiguous empty `None`, member/keyword boundaries and original-range normalization comparison; no production project/IDE provider. |
| pyrefly (CLI) | `1.4.0.dev3`, uv dev group; parity oracle only | 2026-10-03 | `uv run --no-sync pyrefly --version` **passed**: 1.4.0-dev.3 after scoped uv sync; native/CLI parity pending |
| embedded Ruff library crates | registry `=0.0.14`, only Pyrefly's native adapter; workspace `ruff_python_ast`, `ruff_python_parser`, `ruff_source_file`, `ruff_text_size` | 2026-10-03 | Pyrefly dev3 manifests and Cargo.lock read; no salsa feature on this family. Equal bytes do not make its AST the canonical latest AST |
| latest independent Ruff/ty library family: `ruff_*`, `ty_*` | Ruff tag `0.16.10`, mostly Rust crates `0.0.16`, `ruff_linter` **0.16.10**; one fork rev `f7bdff69e1fb94ab0ed5b340e977aac0d26e9301`, parent `3265ed1f944c98bb4c04d632fbefb1257cdb583d`; `third_party/ruff-0.16.10.patch`, sha256 `b7154806d8d5106f02c225d35c4cb8802c869d551af4ce1dd2d391178deaa6f8` | 2026-10-03 | Exact upstream/fork manifests read, remote peel verified; isolated Checker/parsed reuse/cancellation/lint-parity and ty precision controls **passed**. Public explicit-source lint constructor separate-process controls **passed**; scoped fork policy `uv run --no-sync python scripts/check_ruff_fork.py --checkout /home/paul/.cache/lctx-code-facts/ruff-fork` **passed**. Coordinator owns production qualification |
| latest Ruff workspace adapters | `ruff_linter`, `ruff_python_ast_latest`, `ruff_python_parser_latest`, `ruff_text_size_latest`, `ruff_index_latest`, `ruff_python_codegen_latest`, `ruff_python_index_latest`, `ruff_python_semantic`, `ruff_ranged_value`, `ruff_python_stdlib`; flow aliases `ruff_python_ast_ty`, `ruff_python_parser_ty`, `ruff_text_size_ty`, `ruff_db`; same exact fork source | 2026-10-04 | Exact source manifests and all-features Cargo metadata read; named aliases retain crates 0.0.16 at the same fork. Family policy records unpublished ty IDE/project 0.0.0 separately and allows latest-family feature unification in the stub `lctx-workspace-hack`. `uv run --no-sync python scripts/check_family.py Cargo.lock` and 26 scoped checker controls **passed**; current all-features metadata confirms IDE/project remain outside workspace normal/build closure |
| salsa, salsa-macros, salsa-macro-rules | exactly `0.28.5` | 2026-10-03 | Fork manifest and updated Cargo.lock read; latest ty requires this line. Embedded Ruff's salsa feature stays off |
| blake3 | `=1.8.6` | 2026-10-03 | Unchanged exact workspace and Pyrefly requirement; Cargo.lock read |
| analyzer git sources | `github.com/paul-heyse/pyrefly`, `github.com/paul-heyse/ruff`; immutable reviewed revisions above | 2026-10-03 | Updated Cargo.lock; unused yangdanny97/lsp-types retired, upstream now uses published gen-lsp-types 0.11.0 |


## Analyzed libraries (ADR-0117; the pilot per ADR-0021)

Each analyzed library pins itself in `libraries/<name>/` (`pyproject.toml`, `.python-version`,
`uv.lock`); these rows only summarize. `uv lock --project libraries/<name> --check` confirms a lock.

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (analyzed, the pilot) | `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`; release `fastmcp`, `fastmcp-slim`, `fastmcp-tasks` 4.0.5; `mcp`/`mcp-types` 2.2.0; 109 locked packages; Python 3.14.7. Docs/examples/tests: `PrefectHQ/fastmcp@v4.0.5` (fetched in increment 3) | 2026-09-22 | `libraries/fastmcp/uv.lock` (`uv lock`, uv 0.12.18); `lctx acquire fastmcp` then Stage A verified 275 release files against their `RECORD`s; `just pilot` published a snapshot |
| gold reference | fastmcp skill at **4.0.5** (same install line; release 4.0.5; mcp 2.2.0): capability families (22), evaluation only. 49 reviewed claims validate against 4.0.5 (12 re-hashed, 1 re-reviewed, `fm.inputs.r02.1` rewritten: with no active Context or on a lax server the strictness helper returns `None`, so per-parameter `strict=True` holds); 37/37 runtime cases pass | 2026-09-22 | re-pinned through its own `MAINTENANCE.md` pipeline on a copy (`replay_artifacts.py --apply`, `offline_environments.py`, `acquire.py`, `capture_sources.py`, `full_contracts.py`, `refresh_capture.py`, `build.py`, `verify.py` 12/12 with byte-identical rebuild, `qualify.py`), then swapped in; the 4.0.3 original is archived at `~/skill-work/fastmcp-4.0.3-original`. `just gold` (`scripts/check_gold.py`): ok |

## Analytics crates (ADR-0044; leiden-rs from slice 2.3, fcars from 2.5)

| Component | Pin | Verified | How |
|---|---|---|---|
| leiden-rs | `=0.8.1`, `default-features = false` and no features (ADR-0044: built from the dense index, not the `petgraph` adapter; sequential, no rayon) | 2026-09-23 (slice 2.3) | `Cargo.lock` gains only `leiden-rs 0.8.1` (its rand 0.9, rustc-hash 2 and thiserror 2 were already locked); `lctx-analytics` `build.rs` records it in every invocation's `library_versions`; the LFR and shuffled-input fixtures in `communities::tests` |
| biodivine-lib-bdd | `=0.6.3` (Stage 3 condition kernel, ADR-0082; Rust 1.88 minimum) | 2026-09-24 | `cargo info biodivine-lib-bdd@0.6.3` and its installed `Cargo.toml`/apply/transfer sources; `cargo check -p cpg-schema` added it and `num-rational 0.4.2` to the lock; the standalone Stage 3.0 spike passed 4/4 |
| fcars, bitvec | `=0.2.2` and `=1.0.1`, dev-dependencies of `lctx-analytics` only: fcars is the oracle for our FCA's concept sets (ADR-0044), bitvec builds its relation | 2026-09-23 (slice 2.5) | MIT; the lock gains bitvec, funty, radium, tap and wyz (dev only; rayon was already locked); `fcars_agrees_on_the_concepts` compares concept sets on random contexts |
| odis, bit-set | `=2026.9.1` and `=0.8.0`, dev-dependencies of `lctx-analytics` only; independent finite implication/consequence oracle | 2026-10-04 | Installed `odis-2026.9.1/Cargo.toml` confirms release, public bit-set dependency and AGPL-3.0-only; four independent finite-context oracle controls **passed**, 2026-10-04 ([enrichment K1/K2 receipt](plans/code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint)); production bounds and full gates remain separate. Production kernel retained: the oracle has no equivalent admitted allocation/work contract. Registry `odis 2026.9.1 → rust-sugiyama 0.3.0 → petgraph 0.6.5` is isolated behind the analytics dev edge; current post-Hakari all-features Cargo metadata proves no workspace normal/build reachability and no other ingress. The family checker exempts only that exact graph package when fresh locked/offline metadata establishes the same proof; contamination, changed source and absent-proof controls **passed**. |
| rand, rand_chacha, rand_core | the 0.9 line leiden-rs resolves: rand 0.9.5, rand_chacha 0.9.0, rand_core 0.9.5 (the lock also holds rand 0.8 and 0.10 for other crates) | 2026-09-23 (slice 2.3) | `lctx-analytics` `build.rs` asserts each is locked exactly once on the `0.9.` line and records it; rand does not promise sequences across versions (ADR-0044) |

## Serving and embeddings (ADR-0068; the native executor is proposed in ADR-0025)

| Component | Pin | Verified | How |
|---|---|---|---|
| FastMCP (served) | `==4.0.5` in `python/lctx_mcp` (the workspace `uv.lock`; mcp 2.2.0, pydantic 2.13.5), independent of the analyzed pin | 2026-09-23 | `uv lock`; the lctx_mcp tests negotiate `2026-07-28` (auto) and `2025-11-25` (legacy) |
| vLLM | `0.30.1rc1.dev286+g3d5f4d4cd.sm120.r2`, gpu-stack B3/r2 CPython 3.14 SM120 wheel; SHA-256 `84523e185e26680ec202e52f3d2bad73aa77ad0b9fbbc4126f46b9bf37debf64`. Explicit flat index `file:///home/paul/wheelhouse/gpu-stack`; locked torch `2.14.0+cu132`, torchvision `0.29.0+cu132`, CUDA runtime `13.4.2`, Triton `3.8.0`, FlashInfer `0.7.0`; torchaudio excluded (ADR-0080) | 2026-09-28 | Wheel bytes hashed; `uv lock --project services/vllm`; `uv sync --project services/vllm --locked`; installed metadata and CUDA availability read from the service environment |
| Qwen/Qwen3-Embedding-8B | Local `/home/paul/wheelhouse/gpu-stack/models/Qwen3-Embedding-8B-NVFP4-r2`, NVFP4 W4A4 derived from upstream `1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`; manifest SHA-256 `13ceecfe1ab2045c55a8ce39bf0be3b8a526fecdd69c94582a474c85aef08967` binds weights and tokenizer. BF16 activations; MRL 4096→1024, float32/L2 output (ADR-0080) | 2026-09-28 | Controlled launcher verifies the published manifest and all file checksums; basic startup/output receipt in [PR4 evidence](design_review/evidence/2026-09-28_pr4/README.md). No accuracy assessment |
| numpy | `==2.4.6` (vectors, exact cosine; bm25s's only dependency) | 2026-09-23 | `uv lock`; `uv run python -c "import numpy"` |
| httpx2 | `==2.13.1` (the query embedder; bytes sent as `content=`, never `json=`). pydantic's maintained continuation of httpx (operator, 2026-09-24); FastMCP 4.0.5 already depends on it, so the switch removes `httpx` 0.28.1 from the server's lock | 2026-09-24 | PyPI JSON read 2026-09-24 (2.13.1, 2026-09-23; `import httpx2`); `uv lock`; `test_the_http_client_sends_the_exact_bytes_and_reports_a_down_service` |
| bm25s | `==0.3.11`, numpy backend; it depends on numpy only, so no scipy enters the environment | 2026-09-23 | `uv.lock` (`dependencies = [numpy]`); `test_bm25_scores_are_the_lucene_formula` |
| uv_build | `>=0.12,<0.13` (the `lctx-mcp` build backend, matching uv 0.12.18) | 2026-09-23 | `uv sync` built the member |
| PyO3 | `=0.29.2` with `extension-module` (native semantic query member; CPython 3.14.7) | 2026-09-24 | `cargo info pyo3@0.29.2`, official PyO3 0.29.2 changelog for Python 3.14 support, and `cargo check -p lctx-semantics` |
| maturin | `==1.15.0` (the `lctx-semantics` PEP 517/660 backend) | 2026-09-24 | PyPI 1.15.0 release (2026-08-24), `uvx --from maturin==1.15.0 maturin --version`; mixed package route checked in Context7 `/pyo3/maturin`; `uv sync` passed. A one-time `uv build --package lctx-semantics` and isolated CPython 3.14.7 wheel import passed before the operator chose the faster design track; those artifacts were removed and wheel builds are deferred to release. |
| LanceDB | 0.39.0 (Python) — **conditional alternative** beyond selected pgvector | 2026-09-22 (source read at tag) | hybrid/FTS/RRF chain; bundles Arrow 58 / DataFusion 54 |

## Dev tools

| Tool | Version | Verified | How |
|---|---|---|---|
| Python | 3.14.7 (`.python-version`, uv default) | 2026-09-22 | `uv run python --version` |
| ruff | 0.16.10 (uv dev group) | 2026-10-03 | `uv run --no-sync ruff --version` **passed** after uv sync --no-install-workspace |
| pyrefly | 1.4.0.dev3 (uv dev group, parity/type-check CLI) | 2026-10-03 | Scoped uv sync and `uv run --no-sync pyrefly --version` **passed** |
| pytest | 9.1.1 (`uv.lock`) | 2026-09-22 | `uv sync` |
| Hypothesis | 6.168.1 (uv dev group; runtime soundness oracle only) | 2026-09-24 | `uv add --dev 'hypothesis==6.168.1'`; `uv.lock` resolves sortedcontainers 2.4.0; generated programs execute under CPython 3.14.7 in an isolated worker |
| cargo-nextest | 0.9.144 | 2026-09-22 | `just doctor` |
| cargo-hakari | 0.9.39 | 2026-09-28 | `cargo hakari --version`; exact-release upstream configuration source and generated CLI feature union; `just build-features` / `just deps` (ADR-0079) |
| sccache | 0.17.0 (required by `.cargo/config.toml`) | 2026-09-24 | `sccache --version`; cache wrapper; target-environment effect tested 2026-09-28 (cache evidence) |
| cargo-insta | 1.48.0 | 2026-09-22 | `just doctor` |
| cargo-deny | 0.20.2 (sees dev-only duplicates only for named crates; see ADR-0118) | 2026-09-22 | tested with a synthetic duplicate |
| ast-grep | 0.45.3 | 2026-09-22 | `just doctor` |
| just | 1.58.0 | 2026-09-22 | `just --version` |

## Documentation tooling

The single version declaration is [site.toml](site.toml), consumed by local bootstrap, doctor and
CI. Verified 2026-09-25 with `mdbook --version`, `pagefind --version` and `lychee --version`.
Python retains `.python-version`; isolated script tests use pytest from `uv.lock`. Qualification
is `just docs-test` plus `just docs-check` ([publishing operations](publishing.md)).

## PostgreSQL services (ADR-0078; pins verified 2026-09-27, current setup tested 2026-09-28)

| Component | Pin / enabled features | Verification |
|---|---|---|
| PostgreSQL application service | installed 18.6, server_version_num 180006 | actual service query through SQLx; PG18-only startup assertion |
| SQLx / sqlx-cli | =0.9.0; defaults off; postgres, runtime-tokio, macros, migrate, tls-rustls-ring-native-roots | registry manifests, Cargo.lock, `sqlx --version`, assembled Rust check; no enabled MySQL/SQLite driver |
| testcontainers-modules | =0.15.0; defaults off, postgres | registry manifest/Cargo.lock; reexported Testcontainers 0.27.3, real server 180006 |
| Disposable image | postgres:18.6-bookworm@sha256:3725f4e2499eef5134592b3b4ab79a543ed7f8e533b05b5b637af926630f6650 | `docker pull`; single machine-readable pin in `specs/postgres-image.txt` |
| tracing | =0.1.44 (already resolved family; now direct for cache telemetry) | registry manifest and Cargo.lock; matches capability skill |
| Backup tools | pg_dump / pg_restore 18.6 | `pg_dump --version`; `pg_restore --version`; disposable restore drill |
| pgvector Rust | =0.4.2, defaults off, `sqlx` | Exact registry manifest: SQLx 0.9 adapter; real PG18 type, width and signed-zero round trip |
| pgvector extension | 0.8.6; Ubuntu package `postgresql-18-pgvector=0.8.6-1.pgdg24.04+2` | Current bootstrap and real disposable schema/version assertions; extension lives in `lctx_ext` |
| Extension test image | `pgvector/pgvector:0.8.6-pg18-trixie@sha256:78bf48b801e792f99e3ac62b5036fd3876e9be48afda16c1e331af1c75ceb2ff` | Docker pull/digest, `specs/postgres-vector-image.txt`; the plain PostgreSQL image serves current cache/operations tests |
| bytes | =1.12.1 | Shared SQLx/pgpq COPY buffer type; existing resolved version, now direct |
| pgpq | =0.12.0 | Exact registry manifest, Arrow 59 native; real binary COPY and declared Arrow reconstruction |
| pyo3-async-runtimes | =0.29.0, defaults off, `tokio-runtime`; existing PyO3 =0.29.2 | Exact registry API/manifest, `uv sync` native wheel build; one lazy configured two-worker Tokio runtime |
| async-trait | =0.1.92 | 2026-09-28: existing resolved registry release, now direct for DataFusion provider traits |
| datafusion-table-providers-postgres | Owned fork `paul-heyse/datafusion-table-providers`, branch `lctx/df55-stage-reads`, rev `a41da224caf9cae4feff48764d5f6df5ffcb4933`, defaults off, no `federation` (the workspace entry for `common` disables its defaults) | Based on `CaptainEureka` migration `33095588fcdd17301a5d1c340dcd66cd60e41ec8`; prior bounded-pool base `09cc8a8f7e5401ad4424e47247e17f5b74cbe86b`. Verified 2026-09-30: local fork manifests retain version 0.13.1 and DataFusion 55.1. `cargo check -p datafusion-table-providers-postgres --no-default-features` and release `--test bound_lifecycle` passed in the fork (3 real PostgreSQL controls). Adds shared Closing/Closed state, terminal close with drain/release, cancellation guard before `query_raw`, and reservation retention through drain, and terminal abort on failed drain; retains bounded chunks and no reconnect. Patch regenerated from migration base. Workspace focused reader qualification is tracked in the Phase 3 plan; full pin gates wait until Q. |
| tokio-postgres | =0.7.18 | Provider-only configuration adapter; exact registry manifest. SQLx retains application write/transaction ownership |

Psycopg/SQLAlchemy, native ADBC and pgrx remain uninstalled. SeaQuery 1.0.2 is also a direct dependency of the semantic generation DDL lowering
(ADR-0086); it does not own domain definitions or effects. PG15 admits the two declared report views, a closed expression policy and generation-qualified key joins; unsupported expressions remain local. Conditional triggers remain in the [PostgreSQL plan](plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-adoption).
Operation/configuration: [PostgreSQL runbook](postgresql.md). Focused and integrated results:
[PG8–PG11 evidence](design_review/evidence/2026-09-27_postgresql-expansion/implementation.md).

### Catalog wire contracts (ADR-0073), 2026-09-28

Schemars **1.2.2**, direct `std`/`derive`, and Rust jsonschema **0.58.2**, dev-only with
`default-features=false`, were source-verified from the registry `Cargo.toml` (Schemars local
registry; jsonschema exact crate archive). Context7 supplied API routes; source supplied pins.
Offline validation uses explicit draft2020-12 and annotation-only formats. Family pins are
unchanged. Focused schema/native controls passed; complete qualification is recorded by forward-plan PR2.

| Dependency | Direct pin/features | Verified | Source and scope |
|---|---|---|---|
| schemars | =1.2.2; std, derive; default-features=false | 2026-09-28 | Registry Cargo.toml and generated input/output schema controls; PR2 wire and existing format owners |
| jsonschema | =0.58.2; dev-only, default-features=false | 2026-09-28 | Registry Cargo.toml; offline conformance for shared fixtures and existing DTOs |

### Typed semantic model lowerings (ADR-0085/0086), 2026-09-29

Pins verified from the exact local registry `Cargo.toml` in this session. Arrow/DataFusion,
SQLx and analyzer versions did not change. No capability-skill version evidence was extrapolated.

| Dependency | Direct pin/features | Source and focused evidence |
|---|---|---|
| serde_arrow | =0.15.1, arrow-59 | Exact registry manifest and arrow_impl.rs; explicit-schema typed round trip, fixed binary IDs/digests, optional values and primitive lists in `cargo test --release -p lctx-model --test domain` |
| sea-query | =1.0.2, defaults off, backend-postgres/postgres-array | Exact registry manifest, PostgreSQL DDL tests and column API; generation tables/constraints lowered from the validated model |
| syn | =2.0.119, full | Exact registry manifest; bounded Domain/DomainCode derives compile |
| quote | =1.0.47 | Exact registry manifest; derive output is compiled in production domain declarations |

The P0–P2 assembled qualification subsequently passed on 2026-09-30; its bounded receipt is in
the cutover plan §4.2. Phase 3 dependency and assembled checks have their own receipt in the
Phase 3 detailed plan §11. These pin observations alone do not qualify a phase.

## Library skills for the catalog's adjacent crates (built 2026-09-29)

Shared skills (`.config/library-skills.toml`, `~/.local/share/library-skills/skills/<name>`), each
pinned to the crate versions below. Check a claim against the resolved features here before
transferring it: skills index a **named profile**, and the workspace-hack crate (cargo-hakari)
widens some features in our resolved graph.

| Skill | Indexes | Our pin | Delta to check | Verified |
|---|---|---|---|---|
| `sqlx-postgres` | sqlx 0.9.0 (5 crates), sea-query 1.0.2, pgpq 0.12.0, pgvector 0.4.2, testcontainers-modules 0.15.0 + testcontainers 0.27.3, the DataFusion 55 provider fork at git `09cc8a8` (crate 0.13.1) | same registry versions; provider fork `a41da22` | The skill fork profile remains `09cc8a8`; terminal Closing/Closed and early-request drain changes require the new fork source and `bound_lifecycle` tests, not the old skill receipt. sqlx: skill `plain` profile is defaults off with postgres/runtime-tokio/macros/migrate/tls-rustls-ring-native-roots, as declared, but `sqlx-core` also resolves `any`, `json` and `offline` here. sea-query: declared defaults off, resolved with defaults plus chrono/time/bigdecimal/rust_decimal (from the provider crates and the hack crate). pgvector: crate 0.4.2 is not the extension (0.8.6). testcontainers 0.28 cannot resolve with testcontainers-modules 0.15 | 2026-09-29: `verify.py` 14/14, 86 probes confirmed on PostgreSQL 18.6 + pgvector 0.8.6 |
| `pyo3` | pyo3 0.29.2 (+ ffi, macros, macros-backend, build-config), pyo3-async-runtimes 0.29.0 | the same | features here: `extension-module` (with default `macros`), `tokio-runtime`; docs.rs builds pyo3 with `full` and no defaults; probes ran on Python 3.14.7 | 2026-09-29: `verify.py` 13/13, 66 probes and 228 matrix cells confirmed |
| `serde-arrow` | serde_arrow 0.15.1 + marrow 0.3.1, `arrow-59` profile | the same, feature `arrow-59` | the Arrow feature must equal the Arrow pin (`arrow-array`/`arrow-schema` 59.3.0); docs.rs shows the `arrow-60` build | 2026-09-29: `verify.py` 12/12, 37 probes confirmed |
| `fixedbitset` | fixedbitset 0.5.7, default `std`, optional `serde` | the same, `std` only (petgraph's own dependency) | the `serde` claims need a feature we do not enable | 2026-09-29: `verify.py` 12/12, 11 probes confirmed |
| `python-analyzers` | pyrefly 1.4.0-dev.3 (`80cec3f5`) with embedded Ruff 0.0.14; Ruff 0.16.10 (`3265ed1f`), library crates 0.0.16 and ty 0.0.16; salsa 0.28.5 | Pyrefly fork `63cda076`, embedded registry Ruff 0.0.14; independent Ruff/ty fork `f7bdff69`; salsa 0.28.5; dev CLIs Ruff 0.16.10 and Pyrefly 1.4.0.dev3 | Base versions agree. Repository forks add observational seams; their exact patch and scoped receipts are above. Historical skill project-layer anchors and usage snapshots may predate this migration; verify against current source before transferring a repository-use claim | 2026-10-03: skill inventory/probe receipts predate production integration; coordinator owns current producer/consumer qualification |

