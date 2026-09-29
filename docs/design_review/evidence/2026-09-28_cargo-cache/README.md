# Cargo target environment and sccache audit

**Tested within the boundary below, 2026-09-28; ADR-0079.** The operator requested an applicability check
of the PSE-arrow cache-key fix before proceeding with product work. This audit uses library-context
commit `5626941`, Rust 1.98.1 and sccache 0.17.0. The initial audit was read-only. The adopted correction below supersedes its proposed dispositions;
the original cache-key measurements remain valid.

## Result

The reported cache-key mechanism applies here. In an isolated private sccache server/cache per
case, two standalone Cargo projects compiled the registry dependency `itoa =1.0.18` with this
repository's O2 workspace/O3 dependency and Clang/mold settings. All six compilations produced the
same dependency filename and SHA256. The second project results were:

| Target selection | Rust cache hits | Rust cache misses |
|---|---|---|
| Export each project's absolute `CARGO_TARGET_DIR` | 0 | 1 |
| Leave `CARGO_TARGET_DIR` unset | 1 | 0 |
| Leave it unset; pass a distinct non-default `build.target-dir` through Cargo `--config` | 1 | 0 |

**passed:** `uv run --no-sync python docs/design_review/evidence/2026-09-28_cargo-cache/probe.py`.
[Source](probe.py), [raw counts and artifact hashes](raw/probe.json), [output](raw/probe.log).
Each case's first project missed as expected. The binary itself is intentionally non-cacheable;
counts above concern the dependency. One build job, no downloads, no GPU, no workspace build and no
`cargo clean` were used. Only probe-owned private servers were stopped; their temporary caches,
projects and targets were deleted. This establishes the cache-key effect, not a full-workspace
speedup or universal reuse for source-path-sensitive crates.

## Adopted correction

ADR-0079 owns the current policy. `scripts/build_environment.py` normalizes inherited target
exports for ordinary and shebang `just` recipes, SQLx, and both runtime oracle build helpers.
Empty/default/foreign exports are removed; internal custom targets remain intentional, and
`LCTX_CARGO_TARGET_DIR` explicitly overrides external targets. Bare Cargo/uv/IDE shells can run
`eval "$(python3 scripts/build_environment.py --shell)"`. Target paths selected through Cargo config
avoid the environment component of the cache key. The benchmark runner writes both artifact paths
into each disposable trial's Cargo config, including nested maturin builds. It retains captures,
results and pre-recovery targets. No existing benchmark campaign or GPU service was changed.

The toolchain is `nightly-2026-09-29`; Cargo workspace feature unification and fine-grain locking
are enabled. The latter implies the new layout. Metadata reports final output in local `target/`
and intermediates in `/home/paul/.cargo/build/library-context`. `.config/hakari.toml` generates the
CLI feature-union crate; thin library/native boundaries remain excluded. `cargo tree` on CLI,
storage and semantics selections shows equal common target Serde/serde_json features. Semantics
and storage contain no DataFusion/Pyrefly/ty dependency. Host variants remain distinct.

The first nightly build failed in allocative 0.3.6 with overlapping `Infallible` / `!` trait impls.
The local source patch backports the exact eight-line removal from upstream commit
`9711293c6de502d50583cafb12e4a7b764094d3a`; its provenance is in
`third_party/allocative/README.lctx.md`. No third-party version changed. Cargo.lock adds the local
workspace-hack and redirects allocative to that source; all other versions and source pins remain.

A bounded independent review identified two normalization bypasses: oracle helpers re-exported the
default path, and shebang recipes bypassed `set shell`. Both use the shared normalizer now. Review also caught `cargo fmt --all` traversing local path
dependencies: recipes now use the verified virtual-root default, covering all workspace members
and excluding the third-party patch.
A1–A3 otherwise passed within build-tool scope. Product/behavioral gates are outside this review.

## Verification boundary

- **passed:** `uv run --no-sync pytest tests/scripts/test_build_environment.py tests/scripts/test_build_measurements.py -q` (13 tests).
- **passed:** `cargo metadata --offline --no-deps --format-version 1`; `cargo hakari generate --diff`;
  `cargo hakari manage-deps --dry-run`; `scripts/check_family.py Cargo.lock`; `just adr lint`.
- **passed:** isolated two-checkout shared-build probe using the pinned nightly and repository Cargo
  configuration. The second checkout reused the identical intermediate `itoa` artifact as Cargo
  `fresh`; both concurrent warm builds reused it and executed their own final binaries.
  [Runnable probe](shared_probe.py), [raw receipt](raw/shared-probe.json). This is
  bounded sharing/locking evidence, not a timing benchmark or whole-workspace concurrency proof.
- **failed (expected model limit):** standalone `cargo hakari verify` models excluded members
  independently and reports remaining variants. It is not the acceptance gate for Cargo's nightly
  workspace resolver. The generated-file/edge checks are the maintained Hakari gate.
- **passed:** metadata from a nested binding directory in a disposable benchmark copy, on stable
  1.98.1 and pinned nightly: both paths stay inside the trial. No benchmark was executed.
- **passed:** `cargo build --release --locked -p lctx -p lctx-semantics -p lctx-storage`, then
  `cargo build --release --locked -p lctx` and `target/release/lctx --help` after the final lint fix.
- **passed:** `uv sync --locked --reinstall-package lctx-semantics --reinstall-package lctx-storage`;
  both native imports succeed and `readelf -p .comment` identifies rustc 1.101.0-nightly
  (`c1070d693`) and mold 2.42.1 in both installed extensions.
- **passed:** `just fmt`; full `just test-all`: 475 ordinary Rust tests, 199 Python tests,
  19 real-PG Rust tests, three real-PG Python tests, fixture generation, Clippy/Ruff/Pyrefly,
  rules, ADR/agent checks, dependency/gold policy and SQLx metadata against disposable PG18.
  The full invocation completed successfully. Subsequent generated-crate lint policy and formatter
  selection were checked with focused Clippy, a final CLI build and `just fmt-check`.
- **passed:** `just deps`; `cargo clippy --release --locked -p lctx -p lctx-workspace-hack --all-targets --quiet -- -D warnings`;
  `just docs-check` (168 canonical pages, zero errors); `git diff --check`.
- **passed:** source comparison and lockfile audit: only the two-file/eight-line upstream
  allocative change and the new workspace-hack package/source redirect; no version upgrade.
  [Source diff and package identities](raw/adoption-source.json).
- **Observed:** `sccache --show-stats` on the active default endpoint reported 9 Rust hits and
  1,467 misses during qualification, cache location `/home/paul/.cache/sccache`, 100 GiB limit.
  These shared cumulative counters are not an attributable timing or hit-rate benchmark.
  Detailed build/gate logs live in ignored `build/cargo-adoption/`.
- **not_run:** performance benchmarking, a new full-library pilot and live embedding tests. The operator's benchmark remains
  uninterrupted. No `cargo clean` or deletion of captured benchmarks was performed.

## Remaining limits

The exact [sccache 0.17.0 source](https://github.com/mozilla/sccache/blob/v0.17.0/src/compiler/rust.rs)
hashes `CARGO_TARGET_DIR`. It hashes most `CARGO_*` variables, with explicit exceptions including
jobserver, registry configuration, job count and already represented encoded rustflags. Thus “every
CARGO variable” is slightly broader than the implementation. Its
[Rust limitations](https://github.com/mozilla/sccache/blob/v0.17.0/docs/Rust.md) still exclude incremental
compilation, linker-invoking crate types and metadata-only checks. Our O3 non-incremental dependency
profiles are already appropriate; workspace incremental state intentionally remains enabled under
ADR-0079. Final outputs and source-path-sensitive intermediates can still consume per-checkout disk.

Cargo's [build-directory setting](https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir)
is stable. [Workspace feature unification](https://doc.rust-lang.org/cargo/reference/unstable.html#feature-unification)
and [fine-grain locking](https://doc.rust-lang.org/cargo/reference/unstable.html#fine-grain-locking)
remain unstable. Shared clean operations affect every checkout. Feature union can mask missing
feature declarations in a standalone crate; it does not merge host/target, profile or rustflag
variants. Workspace source-path-sensitive units may remain distinct. No full-workspace speedup
has been established. Retained earlier benchmark evidence remains available under its original
capture and result paths.
