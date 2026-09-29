---
id: ADR-0079
title: Share Cargo intermediates and dependency features on a dated nightly
status: accepted
date: 2026-09-28
supersedes: [ADR-0075, ADR-0026]
superseded-by: null
design: [§1.2]
evidence: Tested
revisit: A toolchain update breaks shared builds, a new dependency crosses an ownership boundary, or ordinary rebuild latency warrants paired measurements.
---

## Context

The operator approved the PSE-arrow build measures on 2026-09-28, including unstable Cargo
features, and permits cache rebuilds while preserving benchmarks. The
[cache probe](../design_review/evidence/2026-09-28_cargo-cache/README.md) reproduced cache-key
fragmentation from exported absolute target paths. CLI and Python selections also resolved
common dependencies with different features. [DESIGN §1.2](../design/DESIGN.md#section-1-2)
owns the development contract; Cargo owns resolution and artifact reuse, not application code.

## Options

1. Normalize target exports only and retain stable Cargo. This fixes the demonstrated cache-key
   defect with the least configuration, but retains feature variants and independent intermediates.
2. Use a dated nightly, Cargo workspace feature unification and a shared intermediate directory
   with fine-grain locking; materialize the executable's feature union using Hakari (selected).
   Cargo supplies these mechanisms, including cross-process locking. No bespoke cache is needed.
3. Attach the generated Hakari union to every library. This would import compiler and storage
   dependencies into the pure Python executor and lower contracts. Reject that dependency coupling.
4. Share the final target directory or add another profile. These add output/lock contention or
   another artifact set without improving the selected contract.

## Decision

- Pin `nightly-2026-09-29` (rustc 1.101.0-nightly) and cargo-hakari 0.9.39. The manifest's
  `rust-version = 1.98.1` remains a minimum language requirement, not the development toolchain.
  Enable Cargo `feature-unification = "workspace"` and `fine-grain-locking`, which implies the
  new build-directory layout. Retain 16 jobs and the default single frontend thread.
- Backport upstream allocative commit `9711293c6de502d50583cafb12e4a7b764094d3a` to
  the published 0.3.6 source under `third_party/allocative`, selected by `[patch.crates-io]`.
  Its duplicate never-type implementation conflicts with `Infallible` on this compiler.
  Keep every other dependency version fixed; remove the patch when a compatible release exists.
- `.cargo/config.toml` owns intermediates at `{cargo-cache-home}/build/library-context`, shared
  only among this repository's checkouts. Final binaries and extensions remain in each `target/`.
  Keep sccache and the Clang/mold linker route. No routine `cargo clean`: it would remove shared
  intermediates too. Preserve benchmark captures, source snapshots, results and recovery artifacts.
- Keep workspace O2 incremental dev/release profiles, release 16 codegen units, imported O3
  non-incremental dependencies (including non-workspace path dependencies), and release tests.
  Existing shared sccache capacity remains 100 GiB; no service restart or cache purge is required.
- `scripts/build_environment.py` owns target-export normalization for `just`, SQLx and optional
  interactive activation. Remove empty, default and foreign target exports; preserve intentional
  non-default targets inside the checkout. `LCTX_CARGO_TARGET_DIR` explicitly selects an external
  target. Prefer Cargo configuration for such overrides because exported `CARGO_*` paths still
  participate in sccache keys. Bare Cargo/uv users clear inherited exports with its `--shell` mode.
- `.config/hakari.toml` and the generated `lctx-workspace-hack` materialize feature union at `lctx`
  only. Lower libraries and native bindings do not depend on the hack. The second Ruff family
  stays behind `cpg-flow`. Cargo's workspace resolver unifies the existing dependencies for all
  package selections. `just build-features` regenerates; the existing dependency check verifies
  generated output and managed edges. Cargo.lock and root exact pins retain version ownership.
- Benchmark tooling configures both final and intermediate paths inside each disposable trial
  copy, including nested maturin builds. Source captures stay immutable. Stable/nightly comparison
  variants remain available as benchmarks; their existence does not choose production policy.
- Retain ordinary work on shared `main`, separate worktrees only for truly parallel production
  edits, and isolated benchmark snapshots under ignored `build/`. Retain the light process:
  DESIGN authority, ADR rationale, declared review cadence, focused compile/tests during execution,
  `just fmt` and `just test-all` at scope completion, canonical AGENTS and explicit outcome labels.

## Consequences

Nightly instability is accepted. One feature set does not mean one compilation: host/target,
profiles, toolchains and rustflags can still differ. Workspace feature union can mask missing
feature declarations, so independently publishing a crate requires separate dependency qualification.
Hakari's standalone verifier models excluded members independently and does not certify complete
unification here; the actual nightly Cargo graph is the acceptance boundary. Thin native bindings
retain their dependency boundaries. No end-to-end speedup is claimed without representative timings.

Implementation and command outcomes are recorded in the linked cache evidence. This build decision
does not advance product qualification or waive product tests. Live embedding tests remain waived
by the operator while the critical GPU benchmark runs. Superseded records are retired after their
surviving process/profile clauses move here; benchmark evidence and artifacts remain retained.
