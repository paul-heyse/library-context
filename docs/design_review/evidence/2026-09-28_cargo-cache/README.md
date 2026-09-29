# Cargo target environment and sccache audit

**Tested, 2026-09-28; corrective changes Proposed.** The operator requested an applicability check
of the PSE-arrow cache-key fix before proceeding with product work. This audit uses library-context
commit `5626941`, Rust 1.98.1 and sccache 0.17.0. Production build configuration, toolchain pins,
shared cache contents, active services and existing target directories were not changed.

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

## Current exposure

- The current shell has no `CARGO_TARGET_DIR`, `CARGO_BUILD_TARGET_DIR` or `CARGO_BUILD_BUILD_DIR`.
  There is no repository `.envrc` or per-recipe environment wrapper. `.cargo/config.toml` selects
  sccache, 16 jobs and Clang/mold; Cargo resolves the default target to this repository's `target/`.
- An explicit foreign `CARGO_TARGET_DIR` changed `cargo metadata`'s target and build directories
  to that foreign path. The current plain `just` shell does not normalize inherited target
  variables. A stale IDE/agent environment can therefore reproduce the wrong-directory problem.
- `scripts/postgres_check.py:105` exports the absolute default target. It should remove the
  inherited variable and rely on its explicit repository working directory. Metadata-only
  `cargo check` work itself is not sccache-cacheable; eligible dependencies still see the changed key.
- `scripts/build_measurements.py:223` exports a different absolute target for every trial.
  This prevents otherwise compatible cross-trial dependency reuse in its per-variant cache.
  Same-trial warm Cargo artifacts and same-target recovery are different cases and remain valid;
  this finding does not invalidate their controls.
- Ordinary commands in this shell use the default cache at `/home/paul/.cache/sccache`, with a
  100 GiB maximum. They do not select PSE-arrow's dedicated socket/cache. The sampled default
  endpoint had zero executed compilations and only three non-cacheable compiler probes, so its
  counters do not establish an observed ordinary-build hit rate. Use the selected server's stats,
  not another repository's endpoint. [Environment and metadata controls](raw/environment.json).

**passed:** `cargo metadata --offline --locked --no-deps --format-version 1`, normally and with an
explicit foreign target in the child environment; `sccache --show-stats --stats-format=json`.
The foreign-target control queried metadata only and did not compile or write into that target.

## Proposed correction and limits

1. Remove redundant default-target export from SQLx tooling. Explicit non-default benchmark targets
   should be selected through Cargo configuration or CLI arguments, preserving their current
   isolation and receipt identities without exporting target-path variables to rustc.
2. Normalize stale/default target variables at an existing command-entry boundary, and clear stale
   IDE/session exports. Do not silently redirect an intentional external build: give it an explicit
   override. Keep the current physical workspace target, profiles, linker and pinned toolchain.
3. Update agent/runbook guidance so an inherited foreign path is cleared rather than replaced with
   another exported absolute default. No global cache clearing or `SCCACHE_BASEDIRS` setting is
   needed for the demonstrated registry-dependency case.

The exact [sccache 0.17.0 source](https://github.com/mozilla/sccache/blob/v0.17.0/src/compiler/rust.rs)
hashes `CARGO_TARGET_DIR`. It hashes most `CARGO_*` variables, with explicit exceptions including
jobserver, registry configuration, job count and already represented encoded rustflags. Thus “every
CARGO variable” is slightly broader than the implementation. Its
[Rust limitations](https://github.com/mozilla/sccache/blob/v0.17.0/docs/Rust.md) still exclude incremental
compilation, linker-invoking crate types and metadata-only checks. Our O3 non-incremental dependency
profiles are already appropriate; workspace incremental state intentionally remains enabled under
ADR-0075. Artifact duplication on disk remains independent of sccache hits.

**Proposed, separate investigation:** feature unification and shared build directories. Read-only
`cargo tree --offline --locked -e normal,build --prefix none --format '{p}|{f}'` with `--workspace`,
`-p lctx`, `-p lctx-storage` and `-p lctx-semantics` shows different Serde, serde_json and Tokio feature
sets in actual entry points ([output](raw/features.json)). This confirms distinct configurations,
not their measured compile cost. Arrow/DataFusion features already match in the inspected full
workspace and CLI selections; neither belongs to the pure native executor.

Cargo's [build-directory setting](https://doc.rust-lang.org/cargo/reference/config.html#buildbuild-dir)
is stable; [workspace feature unification](https://doc.rust-lang.org/cargo/reference/unstable.html#feature-unification)
and [fine-grain locking](https://doc.rust-lang.org/cargo/reference/unstable.html#fine-grain-locking)
remain unstable, with the latter implying the new layout. A nightly/shared-directory/hakari switch
would change the current ADR-0075/toolchain policy and needs its own measured case and qualification.
It is not required for the demonstrated fix. More feature sharing can also hide missing feature
declarations; a shared physical build directory adds shared clean/locking consequences.

**not_run:** full workspace rebuild, timing benchmark, native zstd cache probe, nightly, hakari,
shared build-directory trial and product gates. None is necessary to establish this bounded issue;
active benchmark work remains untouched. The implementation request has not been expanded beyond
this applicability audit. This evidence remains current until its proposed corrections are decided.
