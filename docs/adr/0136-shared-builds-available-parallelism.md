---
id: ADR-0136
title: Share Cargo builds and use available build and test parallelism
status: accepted
date: 2026-10-08
supersedes: [ADR-0079]
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: A toolchain update breaks shared builds, a dependency crosses an ownership boundary, or ordinary rebuild latency warrants paired measurements.
---

## Context

The operator requires build and test execution to use available parallelism. Concurrency failures
must be corrected in production or test code. The owned native fixture also needs an allocation
appropriate to the development workstation: its selected default is 16 GiB, with an 8 GiB
SurrealDB tracked-memory threshold. [DESIGN §1.2](../design/DESIGN.md#section-1-2) owns the
development contract; the fixture launcher owns disposable server lifecycle and configuration.

This record carries forward the shared-build decisions of retired ADR-0079. ADR-0132 owns exact
dependency pins, ADR-0126 owns assurance scope, and ADR-0134 owns current preparation, maintenance
and worktree commands. Original build evidence remains in the
[cache probe](../design_review/evidence/2026-09-28_cargo-cache/README.md).

## Options

1. Use Cargo and test-runner defaults, and select the pinned compiler's automatic frontend and
   backend worker counts. This lets the tools schedule against available CPUs (selected).
2. Add a repository scheduler or host-specific worker heuristic. This duplicates tool ownership
   and introduces another configuration authority without a demonstrated requirement.

## Decision

- Pin `nightly-2026-09-29` (rustc 1.101.0-nightly) and cargo-hakari 0.9.39. The manifest's
  `rust-version = 1.98.1` is the minimum language requirement. Enable Cargo workspace feature
  unification and fine-grain locking with their implied intermediate-directory layout.
- Use available build/test parallelism. The pinned compiler accepts `-Z unstable-options
  --jobs-frontend=0 --jobs-backend=0`; Cargo owns its jobserver and tests use their ordinary
  worker defaults. Fix concurrency defects in the executing code.
- `.cargo/config.toml` owns intermediates at `{cargo-cache-home}/build/library-context`, shared
  among this repository's checkouts. Final binaries/extensions remain in local `target/`.
  Keep sccache, Clang/mold, workspace O2 incremental dev/release profiles, imported O3
  non-incremental dependencies (including non-workspace path dependencies), and release tests.
  Preserve caches and benchmark captures; never routinely `cargo clean` shared intermediates.
  Existing shared sccache capacity remains 100 GiB.
- Retain the allocative 0.3.6 upstream never-type backport at
  `9711293c6de502d50583cafb12e4a7b764094d3a`, through `third_party/allocative` and `[patch.crates-io]`.
  Remove it when the selected dependency graph supplies a compatible release. Other version
  changes follow ADR-0132.
- `scripts/build_environment.py` normalizes inherited target exports for repository launchers;
  bare Cargo reads the repository configuration. `LCTX_CARGO_TARGET_DIR` selects an intentional
  external target. Stable paths preserve cache identity. ADR-0134 owns scoped environment repair.
- `.config/hakari.toml` and generated `lctx-workspace-hack` materialize the feature union only
  at `lctx`. Lower libraries/native bindings remain outside it. Cargo owns workspace resolution;
  `just build-features` regenerates and `just deps` checks the generated edges. Separate analyzer
  families stay inside their owning adapters. Exact pins and Cargo.lock retain version ownership.
- Benchmark tooling owns final/intermediate directories inside immutable disposable source
  captures, including nested maturin builds. Its explicitly selected comparison variants do not
  prescribe ordinary execution settings.
- Keep ordinary work on shared `main`, with worktrees for independent/conflicting mutable state
  under ADR-0134. DESIGN owns architecture, ADRs rationale, and AGENTS current commands. Review
  cadence and assurance remain with ADR-0040/0126; report actual outcomes and run scoped end-of-turn
  maintenance where concurrent edits require it.
- `scripts/surrealdb_fixture.py` defaults to `16G`; `LCTX_FIXTURE_MEMORY` and `--memory` retain
  explicit overrides. The server threshold is half the selected allocation. Lifecycle ownership,
  OOM reporting, authentication and operator-store isolation remain the fixture's responsibility.

## Consequences

Nightly instability remains accepted. Host/target, profile, toolchain and rustflag differences can
still require separate compilations. Changing worker flags invalidates affected compiled cache
entries. No end-to-end speedup is claimed without measurement. Feature unification can mask missing
standalone feature declarations; independently publishing a crate needs separate qualification.

**Tested, 2026-10-08:** the pinned compiler compiled and ran a small program with automatic workers;
fixture and build-helper Python controls passed; an actual owned server started with the selected
16 GiB allocation and 8 GiB threshold. These controls do not establish the ongoing persisted
execution plan's product acceptance. The [correction plan](../plans/persisted-execution-corrections-plan_2026-10-07.md)
retains that separate scope and its current verification obligations.
