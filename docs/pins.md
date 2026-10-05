# Pins

Dependencies float: manifests carry the tool's default specifier (`uv add` → `>=`, `cargo add`
→ caret), the committed lockfiles record the resolved versions, and `just upgrade` moves them
to the latest at an agent's discretion. This page lists only the deliberate exceptions — an
exact version, cap, git rev or hold-back — each with the reason specific to that dependency and
when to revisit it. Add, change or remove a row with the `pin-check` skill. A dependency with
no row here is whatever the lockfile holds.

A caret `[workspace.dependencies]` requirement needs nothing here. An exact `=x.y.z` or a git
`rev` needs a row; `just deps` checks that it has one (`scripts/check_family.py`). Design authority
for the pinned compute and analyzer families is DESIGN §7 / ADR-0118; the policy is ADR-0125.
A row without a date is not verified.

## Toolchains

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| Rust toolchain | nightly-2026-09-29 (`rust-toolchain.toml`); manifest minimum remains 1.98.1 | Shared builds on one dated nightly (ADR-0079); never a floating `+nightly` or `+stable` | ADR-0079's trigger: a toolchain update breaks shared builds | 2026-09-28: `rustc -Vv`: 1.101.0-nightly, c1070d69382b8d2f2eb65119c738a77d9e324c9e, LLVM 23.1.1; `cargo -V`: 1.101.0-nightly (3d7cf6e93). Installed rustfmt/Clippy |
| Python | 3.14.7 (`.python-version`, uv default) | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: `uv run python --version` |
| just | 1.58.0 or later | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: `just --version` |
| cargo-nextest | 0.9.144 | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: `just doctor` |
| cargo-hakari | 0.9.39 | Interpreter and toolchain versions are outside the float policy for now; generated CLI feature union (ADR-0079) | the policy extends to toolchains | 2026-09-28: `cargo hakari --version`; exact-release upstream configuration source and generated CLI feature union; `just build-features` / `just deps` |
| sccache | 0.17.0 (required by `.cargo/config.toml`) | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-24: `sccache --version`; cache wrapper; target-environment effect tested 2026-09-28 (cache evidence) |
| cargo-insta | 1.48.0 | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: `just doctor` |
| cargo-deny | 0.20.2 (sees dev-only duplicates only for named crates; see ADR-0118) | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: tested with a synthetic duplicate |
| ast-grep | 0.45.3 | Interpreter and toolchain versions are outside the float policy for now | the policy extends to toolchains | 2026-09-22: `just doctor` |

## Compute family and vendored sources (ADR-0118, ADR-0079)

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| datafusion | =55.1.0 | Type-sharing family: one version in the core workspace (§B9) | a deliberate family upgrade (ADR) | 2026-09-29: `family_smoke` (Arrow batches queried by DataFusion SQL) passed; single version in `Cargo.lock`. The delta-rs family and `datafusion-federation` were removed with the Delta store (plan P1.4, ADR-0118) |
| arrow-*, parquet | =59.3.0; `parquet` is a direct dependency since H1 P6 (`default-features = false, features = ["zstd"]`: the codec was already compiled) | Type-sharing family (§B9) | with datafusion | 2026-09-23: same; `data_files_are_zstd` |
| object_store | 0.13.2, held by `Cargo.lock` (no crate depends on it directly, so a workspace pin would pin nothing; H1 O2) and kept single by `check_family.py` | Type-sharing family (§B9) | with datafusion | 2026-09-23: `Cargo.lock`; `cargo shear` in `just deps` |
| petgraph | =0.8.3, default features plus `serde-1`, no `rayon` | Declared family (`check_family.py`); immutable typed graph snapshots serialize its structures (ADR-0103) | a snapshot format change or a deliberate graph upgrade | 2026-09-30: pinned source `graph_impl/serialization.rs` inspected; four focused snapshot controls passed (including 70,000-node multi-chunk roundtrip); full gate at Q |
| postcard | =1.1.3, no defaults, `alloc` | Byte-stable binary graph snapshot wrapper around the petgraph Serde object (ADR-0103) | with petgraph | 2026-09-30: downloaded registry Cargo.toml and `to_slice`/`take_from_bytes`/`experimental::serialized_size` inspected; compile and roundtrip/size/truncation/resource controls passed; full gate at Q |
| serde_arrow | =0.15.1, `arrow-59` | Its Arrow feature must equal the Arrow family (ADR-0085/0086) | with the Arrow family | 2026-09-29: exact registry manifest and arrow_impl.rs; explicit-schema typed round trip, fixed binary IDs/digests, optional values and primitive lists in `cargo test --release -p lctx-model --test domain` |
| pgpq | =0.12.0 | Arrow 59 native: must match the Arrow family | with the Arrow family | exact registry manifest; real binary COPY and declared Arrow reconstruction |
| allocative | 0.3.6 with local upstream never-type backport (`third_party/allocative`, `[patch.crates-io]`) | Vendored source: backports the upstream never-type fix without upgrading Pyrefly's graph (ADR-0079) | Pyrefly's graph reaches an allocative release with the fix | 2026-09-28: published manifest/source and upstream commit `9711293c6de502d50583cafb12e4a7b764094d3a`; eight-line duplicate impl/obsolete feature removal |
| tikv-jemallocator | =0.7.0 (jemalloc 5.3.1): the binaries' global allocator, Linux/macOS | Pyrefly's own allocator dependency at its version; the allocator spike measured this release (ADR-0016) | Pyrefly's allocator requirement moves | 2026-09-23: Pyrefly's Linux/macOS dependency, already compiled; its CLI uses it; the allocator spike's 16 runs and H1's pilots |
| blake3 | `=1.8.6` | Pyrefly requires this exact version; id derivation (DESIGN §3.4.1) | Pyrefly's requirement moves | 2026-10-03: unchanged exact workspace and Pyrefly requirement; Cargo.lock read |

## Analyzers (ADR-0117)

The linked analyzer families are workspace dependencies (`Cargo.toml`, `Cargo.lock`).
These rows record the current pins and their individually scoped verification receipts; the
archived in-process Pyrefly prototype is historical evidence, not the current pin authority.

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| pyrefly (library): `pyrefly`, `pyrefly_build`, `pyrefly_bundled`, `pyrefly_config`, `pyrefly_python`, `pyrefly_types`, `pyrefly_util`, `tsp_types` | 1.4.0-dev.3, fork rev `63cda076956013cd0bd1d0d05c785f747fe6adc0`, exact parent/tag `80cec3f57364bc11d4a39a419f6894a8eabcaa00`; `third_party/pyrefly-1.4.0-dev.3.patch`, sha256 `178788fc11391c4a638306b06ba9433f15a6950dbdc184c6c5042959896e0391` | Fork: observation-only original overload identity and conservative selection state (ADR-0121); fork changes get a new pinned revision | upstream exposes equivalent observational contracts | 2026-10-04: isolated pinned-toolchain compile with existing allocative patch **passed**; three assembled native origin/replay controls and unchanged eight-case old/new types/diagnostics parity **passed**; final PostgreSQL consumers and integrated gates pending. Earlier fork receipts retain their original boundary. Embedded Ruff remains 0.0.14; old immutable refs serve historical checkouts, never runtime fallback |
| ty IDE/reference oracle | `ty_ide`/`ty_project` unpublished 0.0.0, with transitive `ty_python_semantic` 0.0.16, at independent Ruff fork `9080ee7a82a4ec7359ba2ade60839ef1108a2968`; extractor development closure only | Fork; reference oracle | the latest Ruff/ty fork moves | 2026-10-04: predecessor fork `f7bdff69e1fb94ab0ed5b340e977aac0d26e9301` source manifests, release compile and `cargo test --release -p cpg-extract --test python_reference_oracle -- --nocapture` **passed**; current-fork rerun pending. Joined-process hermetic controls retain ResolveAliases requested-spelling filtering, ambiguous empty `None`, member/keyword boundaries and original-range normalization comparison; no production project/IDE provider |
| pyrefly (CLI) | `==1.4.0.dev3`, uv dev group | Parity oracle for the linked native Pyrefly; also the type checker | the native Pyrefly pin moves | 2026-10-03: `uv run --no-sync pyrefly --version` **passed**: 1.4.0-dev.3 after scoped uv sync; native/CLI parity pending |
| embedded Ruff library crates | registry `=0.0.14`, only Pyrefly's native adapter; workspace `ruff_python_ast`, `ruff_python_parser`, `ruff_source_file`, `ruff_text_size` | Declared family: Pyrefly's embedded Ruff, scoped to its adapter (ADR-0118) | the Pyrefly pin moves | 2026-10-03: Pyrefly dev3 manifests and Cargo.lock read; no salsa feature on this family. Equal bytes do not make its AST the canonical latest AST |
| latest independent Ruff/ty library family: `ruff_*`, `ty_*` | Ruff tag `0.16.10`, mostly Rust crates `0.0.16`, `ruff_linter` **0.16.10**; fork rev `9080ee7a82a4ec7359ba2ade60839ef1108a2968`, parent `3265ed1f944c98bb4c04d632fbefb1257cdb583d`; `third_party/ruff-0.16.10.patch`, sha256 `b6bb67f9d297dda222a5b7c983b553424fed37ed6e31e90aa797bde9d0bb9b2e` | Fork: consumed Binding/Reference/context and lint constructor; declared family (ADR-0118) | upstream supplies equivalent observational contracts | 2026-10-04: fork retains consumed Binding/Reference/context and lint constructor; retires unused supplemental Export/Branch. Five-case old/new remaining-fact/node/lint-diagnostic parity **passed**; immutable remote ref verified. Current linked consumer and Q0 qualification pending |
| latest Ruff workspace adapters | `ruff_linter`, `ruff_python_ast_latest`, `ruff_python_parser_latest`, `ruff_text_size_latest`, `ruff_index_latest`, `ruff_python_codegen_latest`, `ruff_python_index_latest`, `ruff_python_semantic`, `ruff_ranged_value`, `ruff_python_stdlib`; flow aliases `ruff_python_ast_ty`, `ruff_python_parser_ty`, `ruff_text_size_ty`, `ruff_db`; same exact fork source | Fork (as above) | with the fork | 2026-10-04: exact source manifests and all-features Cargo metadata read; named aliases retain crates 0.0.16 at the same fork. Family policy records unpublished ty IDE/project 0.0.0 separately and allows latest-family feature unification in the stub `lctx-workspace-hack`. `uv run --no-sync python scripts/check_family.py Cargo.lock` and 26 scoped checker controls **passed**; current all-features metadata confirms IDE/project remain outside workspace normal/build closure |
| salsa, salsa-macros, salsa-macro-rules | exactly `0.28.5` | Type-sharing family required by latest ty (ADR-0118) | the Ruff/ty fork moves | 2026-10-03: fork manifest and updated Cargo.lock read; latest ty requires this line. Embedded Ruff's salsa feature stays off |
| analyzer git sources | `github.com/paul-heyse/pyrefly`, `github.com/paul-heyse/ruff`; immutable reviewed revisions above | Forks | with the forks | 2026-10-03: updated Cargo.lock; unused yangdanny97/lsp-types retired, upstream now uses published gen-lsp-types 0.11.0 |

## Analyzed libraries (ADR-0117; the pilot per ADR-0021)

Each analyzed library pins itself in `libraries/<name>/` (`pyproject.toml`, `.python-version`,
`uv.lock`); these rows only summarize. `uv lock --project libraries/<name> --check` confirms a lock.
These locks move only through `lctx library` and `uv lock --project libraries/<name>
--upgrade-package <dist>` (`libraries/README.md`), never through `just upgrade`.

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| FastMCP (analyzed, the pilot) | `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`; release `fastmcp`, `fastmcp-slim`, `fastmcp-tasks` 4.0.5; `mcp`/`mcp-types` 2.2.0; 109 locked packages; Python 3.14.7. Docs/examples/tests: `PrefectHQ/fastmcp@v4.0.5` (fetched in increment 3) | The analyzed product: the catalog is of this exact release and must match the gold reference (`scripts/check_gold.py`) | a deliberate re-pin of the pilot with the gold skill | 2026-09-22: `libraries/fastmcp/uv.lock` (`uv lock`, uv 0.12.18); `lctx acquire fastmcp` then Stage A verified 275 release files against their `RECORD`s; `just pilot` published a snapshot |
| gold reference | fastmcp skill at **4.0.5** (same install line; release 4.0.5; mcp 2.2.0): capability families (22), evaluation only. 49 reviewed claims validate against 4.0.5 (12 re-hashed, 1 re-reviewed, `fm.inputs.r02.1` rewritten: with no active Context or on a lax server the strictness helper returns `None`, so per-parameter `strict=True` holds); 37/37 runtime cases pass | Library skill; evaluation gold | with the analyzed pilot | 2026-09-22: re-pinned through its own `MAINTENANCE.md` pipeline on a copy (`replay_artifacts.py --apply`, `offline_environments.py`, `acquire.py`, `capture_sources.py`, `full_contracts.py`, `refresh_capture.py`, `build.py`, `verify.py` 12/12 with byte-identical rebuild, `qualify.py`), then swapped in; the 4.0.3 original is archived at `~/skill-work/fastmcp-4.0.3-original`. `just gold` (`scripts/check_gold.py`): ok |

## Analytics crates (ADR-0044)

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| leiden-rs | `=0.8.1`, `default-features = false` and no features (ADR-0044: built from the dense index, not the `petgraph` adapter; sequential, no rayon) | Community assignments are reproducible output; rand does not promise sequences across versions (ADR-0044) | a deliberate community-output change | 2026-09-23 (slice 2.3): `Cargo.lock` gains only `leiden-rs 0.8.1` (its rand 0.9, rustc-hash 2 and thiserror 2 were already locked); `lctx-analytics` `build.rs` records it in every invocation's `library_versions` (stale: no such `build.rs` exists); the LFR and shuffled-input fixtures in `communities::tests` |
| rand, rand_chacha, rand_core | the 0.9 line leiden-rs resolves: rand 0.9.5, rand_chacha 0.9.0, rand_core 0.9.5 (the lock also holds rand 0.8 and 0.10 for other crates) | rand does not promise sequences across versions (ADR-0044) | with leiden-rs | 2026-09-23 (slice 2.3): `lctx-analytics` `build.rs` asserts each is locked exactly once on the `0.9.` line and records it (stale: no such `build.rs` exists) |
| odis, bit-set | `=2026.9.1` and `=0.8.0`, dev-dependencies of `lctx-analytics` only | Independent finite implication/consequence oracle; the family checker exempts only its exact `rust-sugiyama 0.3.0 → petgraph 0.6.5` graph | the oracle's graph or the exemption changes | 2026-10-04: installed `odis-2026.9.1/Cargo.toml` confirms release, public bit-set dependency and AGPL-3.0-only; four independent finite-context oracle controls **passed** ([enrichment K1/K2 receipt](plans/code-facts-analytical-enrichment-plan_2026-10-04.md#7-current-disposition-and-checkpoint)); production bounds and full gates remain separate. Production kernel retained: the oracle has no equivalent admitted allocation/work contract. Registry `odis 2026.9.1 → rust-sugiyama 0.3.0 → petgraph 0.6.5` is isolated behind the analytics dev edge; current post-Hakari all-features Cargo metadata proves no workspace normal/build reachability and no other ingress. The family checker exempts only that exact graph package when fresh locked/offline metadata establishes the same proof; contamination, changed source and absent-proof controls **passed** |

## Serving and embeddings (ADR-0068; the native executor is proposed in ADR-0025)

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| FastMCP (served), mcp-types | `fastmcp==4.0.5`, `mcp-types==2.2.0` in `python/lctx_mcp` (the workspace `uv.lock`; mcp 2.2.0, pydantic 2.13.5), independent of the analyzed pin | Served wire bytes mirror FastMCP 4.0.5; `lctx_mcp.wire` serializes through mcp-types | a deliberate protocol or FastMCP move | 2026-09-23: `uv lock`; the lctx_mcp tests negotiate `2026-07-28` (auto) and `2025-11-25` (legacy) |
| vLLM (`services/vllm`) | `0.30.1rc1.dev286+g3d5f4d4cd.sm120.r2`, gpu-stack B3/r2 CPython 3.14 SM120 wheel; SHA-256 `84523e185e26680ec202e52f3d2bad73aa77ad0b9fbbc4126f46b9bf37debf64`. Explicit flat index `file:///home/paul/wheelhouse/gpu-stack`; locked torch `2.14.0+cu132`, torchvision `0.29.0+cu132`, CUDA runtime `13.4.2`, Triton `3.8.0`, FlashInfer `0.7.0`; torchaudio excluded (ADR-0080) | Custom wheel (content-addressed) and platform availability; the service lock never moves through `just upgrade` | a new gpu-stack wheel | 2026-09-28: wheel bytes hashed; `uv lock --project services/vllm`; `uv sync --project services/vllm --locked`; installed metadata and CUDA availability read from the service environment |
| Qwen/Qwen3-Embedding-8B | Local `/home/paul/wheelhouse/gpu-stack/models/Qwen3-Embedding-8B-NVFP4-r2`, NVFP4 W4A4 derived from upstream `1d8ad4ca9b3dd8059ad90a75d4983776a23d44af`; manifest SHA-256 `13ceecfe1ab2045c55a8ce39bf0be3b8a526fecdd69c94582a474c85aef08967` binds weights and tokenizer. BF16 activations; MRL 4096→1024, float32/L2 output (ADR-0080) | Content-addressed acquisition | a new model derivation | 2026-09-28: controlled launcher verifies the published manifest and all file checksums; basic startup/output receipt in [PR4 evidence](design_review/evidence/2026-09-28_pr4/README.md). No accuracy assessment |
| uv_build | `>=0.12,<0.13` (the `lctx-mcp` build backend) | Upper cap at the uv minor in use, as uv recommends for its build backend | uv moves to 0.13 | 2026-09-23: `uv sync` built the member |
| PyO3, pyo3-async-runtimes | `=0.29.2` with `extension-module`; `=0.29.0`, defaults off, `tokio-runtime` | pyo3-async-runtimes is released in lockstep with PyO3 (type-sharing pair); the `pyo3` library skill indexes exactly these versions | a deliberate PyO3 upgrade, with the skill | 2026-09-24: `cargo info pyo3@0.29.2`, official PyO3 0.29.2 changelog for Python 3.14 support, and `cargo check -p lctx-semantics`; pyo3-async-runtimes: exact registry API/manifest, `uv sync` native wheel build; one lazy configured two-worker Tokio runtime |
| hypothesis | `==6.168.1` (uv dev group; runtime soundness oracle only) | The version the `python-oracles` library skill indexes (library-skill pin, outside this policy for now) | the skill moves | 2026-09-24: `uv add --dev 'hypothesis==6.168.1'`; `uv.lock` resolves sortedcontainers 2.4.0; generated programs execute under CPython 3.14.7 in an isolated worker |

## PostgreSQL services (ADR-0078; pins verified 2026-09-27, current setup tested 2026-09-28)

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| PostgreSQL application service | installed 18.6, server_version_num 180006 | Installed service; PG18-only startup assertion | a PostgreSQL major upgrade | actual service query through SQLx |
| SQLx / sqlx-cli | =0.9.0; defaults off; postgres, runtime-tokio, macros, migrate, tls-rustls-ring-native-roots | Kept pending operator review: SQLx and sqlx-cli are recorded together at one version (ADR-0068); no other reason is recorded | the operator decides | registry manifests, Cargo.lock, `sqlx --version`, assembled Rust check; no enabled MySQL/SQLite driver |
| Disposable image | postgres:18.6-bookworm@sha256:3725f4e2499eef5134592b3b4ab79a543ed7f8e533b05b5b637af926630f6650 | Content-addressed acquisition | the service version moves | `docker pull`; single machine-readable pin in `specs/postgres-image.txt` |
| Backup tools | pg_dump / pg_restore 18.6 | Must match the installed service | with the service | `pg_dump --version`; `pg_restore --version`; disposable restore drill |
| pgvector extension | 0.8.6; Ubuntu package `postgresql-18-pgvector=0.8.6-1.pgdg24.04+2` | Installed service extension (lives in `lctx_ext`) | with the service | current bootstrap and real disposable schema/version assertions |
| Extension test image | `pgvector/pgvector:0.8.6-pg18-trixie@sha256:78bf48b801e792f99e3ac62b5036fd3876e9be48afda16c1e331af1c75ceb2ff` | Content-addressed acquisition | with the extension | Docker pull/digest, `specs/postgres-vector-image.txt`; the plain PostgreSQL image serves current cache/operations tests |
| datafusion-table-providers-postgres | Owned fork `paul-heyse/datafusion-table-providers`, branch `lctx/df55-stage-reads`, rev `a41da224caf9cae4feff48764d5f6df5ffcb4933`, defaults off, no `federation` (the workspace entry for `common` disables its defaults) | Fork: shared Closing/Closed state, terminal close with drain/release, cancellation guard before `query_raw`, reservation retention through drain, terminal abort on failed drain; bounded chunks, no reconnect | upstream provides equivalent bounded lifecycle | 2026-09-30: based on `CaptainEureka` migration `33095588fcdd17301a5d1c340dcd66cd60e41ec8`; prior bounded-pool base `09cc8a8f7e5401ad4424e47247e17f5b74cbe86b`. Local fork manifests retain version 0.13.1 and DataFusion 55.1. `cargo check -p datafusion-table-providers-postgres --no-default-features` and release `--test bound_lifecycle` passed in the fork (3 real PostgreSQL controls). Patch regenerated from migration base. Workspace focused reader qualification is tracked in the Phase 3 plan; full pin gates wait until Q |

Psycopg/SQLAlchemy, native ADBC and pgrx remain uninstalled. SeaQuery is also a direct dependency of the semantic generation DDL lowering
(ADR-0086); it does not own domain definitions or effects. PG15 admits the two declared report views, a closed expression policy and generation-qualified key joins; unsupported expressions remain local. Conditional triggers remain in the [PostgreSQL plan](plans/behavioral-model-forward-plan_2026-09-24.md#postgresql-adoption).
Operation/configuration: [PostgreSQL runbook](postgresql.md). Focused and integrated results:
[PG8–PG11 evidence](design_review/evidence/2026-09-27_postgresql-expansion/implementation.md).

## Catalog wire contracts (ADR-0073), 2026-09-28

Context7 supplied API routes; source supplied pins. Offline validation uses explicit
draft2020-12 and annotation-only formats. Focused schema/native controls passed; complete
qualification is recorded by forward-plan PR2.

| Component | Pin | Reason | Revisit when | Verified |
|---|---|---|---|---|
| schemars | =1.2.2; std, derive; default-features=false | Generated input/output wire schemas are published contracts (ADR-0073) | a deliberate wire-schema revision | 2026-09-28: registry Cargo.toml (local registry) and generated input/output schema controls; PR2 wire and existing format owners |
| jsonschema | =0.58.2; dev-only, default-features=false | Conformance oracle for those schemas | with schemars | 2026-09-28: registry Cargo.toml (exact crate archive); offline conformance for shared fixtures and existing DTOs |

## Documentation tooling

Documentation binaries are listed in [site.toml](site.toml), consumed by local bootstrap, doctor
and CI. A tool listed there without a version means the latest (presence is checked); a version
given there is a pin and needs a row here. Python retains `.python-version`; isolated script
tests use pytest from `uv.lock`. Qualification is `just docs-test` plus `just docs-check`
([publishing operations](publishing.md)).

## Library skills for the catalog's adjacent crates (built 2026-09-29)

Shared skills (`.config/library-skills.toml`, `~/.local/share/library-skills/skills/<name>`), each
pinned to the crate versions below. Check a claim against the locked version (`Cargo.lock`/`uv.lock`)
and the resolved features before transferring it: skills index a **named profile**, and the
workspace-hack crate (cargo-hakari) widens some features in our resolved graph.

| Skill | Indexes | Our pin | Delta to check | Verified |
|---|---|---|---|---|
| `sqlx-postgres` | sqlx 0.9.0 (5 crates), sea-query 1.0.2, pgpq 0.12.0, pgvector 0.4.2, testcontainers-modules 0.15.0 + testcontainers 0.27.3, the DataFusion 55 provider fork at git `09cc8a8` (crate 0.13.1) | same registry versions; provider fork `a41da22` | The skill fork profile remains `09cc8a8`; terminal Closing/Closed and early-request drain changes require the new fork source and `bound_lifecycle` tests, not the old skill receipt. sqlx: skill `plain` profile is defaults off with postgres/runtime-tokio/macros/migrate/tls-rustls-ring-native-roots, as declared, but `sqlx-core` also resolves `any`, `json` and `offline` here. sea-query: declared defaults off, resolved with defaults plus chrono/time/bigdecimal/rust_decimal (from the provider crates and the hack crate). pgvector: crate 0.4.2 is not the extension (0.8.6). testcontainers 0.28 cannot resolve with testcontainers-modules 0.15 | 2026-09-29: `verify.py` 14/14, 86 probes confirmed on PostgreSQL 18.6 + pgvector 0.8.6 |
| `pyo3` | pyo3 0.29.2 (+ ffi, macros, macros-backend, build-config), pyo3-async-runtimes 0.29.0 | the same | features here: `extension-module` (with default `macros`), `tokio-runtime`; docs.rs builds pyo3 with `full` and no defaults; probes ran on Python 3.14.7 | 2026-09-29: `verify.py` 13/13, 66 probes and 228 matrix cells confirmed |
| `serde-arrow` | serde_arrow 0.15.1 + marrow 0.3.1, `arrow-59` profile | the same, feature `arrow-59` | the Arrow feature must equal the Arrow pin (`arrow-array`/`arrow-schema` 59.3.0); docs.rs shows the `arrow-60` build | 2026-09-29: `verify.py` 12/12, 37 probes confirmed |
| `fixedbitset` | fixedbitset 0.5.7, default `std`, optional `serde` | the same, `std` only (petgraph's own dependency) | the `serde` claims need a feature we do not enable | 2026-09-29: `verify.py` 12/12, 11 probes confirmed |
| `python-analyzers` | pyrefly 1.4.0-dev.3 (`80cec3f5`) with embedded Ruff 0.0.14; Ruff 0.16.10 (`3265ed1f`), library crates 0.0.16 and ty 0.0.16; salsa 0.28.5 | Pyrefly fork `63cda076`, embedded registry Ruff 0.0.14; independent Ruff/ty fork `9080ee7a`; salsa 0.28.5; dev CLIs Ruff 0.16.10 and Pyrefly 1.4.0.dev3 | Base versions agree. Repository forks add observational seams; their exact patch and scoped receipts are above. Shared skill is repo-agnostic; current adapter wiring is in acquisition/extraction and this page. Scoped fork receipts transfer only to their recorded source | 2026-10-03: skill inventory/probe receipts predate production integration; coordinator owns current producer/consumer qualification |
