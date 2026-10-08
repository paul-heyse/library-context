---
id: ADR-0137
title: Qualify local test compilation separately and scope execution provenance
status: accepted
date: 2026-10-08
supersedes: [ADR-0136]
superseded-by: null
design: [§1.2, §4, §15]
evidence: Proposed
revisit: Candidate qualification exposes a production regression, source attribution omits a relevant dependency, or remaining build latency warrants a bounded investigation.
---

## Context

The operator requires build and test execution to use available parallelism. Concurrency failures
must be corrected in production or test code. The owned native fixture also needs an allocation
appropriate to the development workstation: its selected default is 16 GiB, with an 8 GiB
SurrealDB tracked-memory threshold. [DESIGN §1.2](../design/DESIGN.md#section-1-2) owns the
development contract; the fixture launcher owns disposable server lifecycle and configuration.

The operator accepted compilation-cost RC01/RC02 and a qualified bare-tool local default on 2026-10-08. The [compilation-cost plan](../plans/rust-compilation-costs-plan_2026-10-08.md) owns BC0–BC5 and F01–F04. Its independent target review accepts the Proposed architecture; implementation and qualification remain open.

This record carries forward all unaffected shared-build decisions of retired ADR-0136 and ADR-0079. ADR-0132 owns exact
dependency pins, ADR-0126 owns assurance scope, and ADR-0134 owns current preparation, maintenance
and worktree commands. Original build evidence remains in the
[cache probe](../design_review/evidence/2026-09-28_cargo-cache/README.md).

## Options

1. Use Cargo and test-runner defaults, and select the pinned compiler's automatic frontend and
   backend worker counts. This lets the tools schedule against available CPUs (selected).
2. Keep release-only local tests and whole-workspace provider fingerprints. This preserves the current loop but retains optimizer work and unrelated downstream invalidation.
3. Keep a permanent opt-in local profile. This makes selection explicit but duplicates a standing default. A temporary candidate followed by the qualified test default serves the operator better.
4. Add a repository scheduler, alternate row model or a second dependency resolver. These duplicate tool/model ownership without removing the diagnosed coupling.

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
  non-incremental dependencies (including non-workspace path dependencies). Qualify an explicit
  O1/incremental/debug-zero/LTO-off test candidate with assertions and overflow checks enabled.
  After normal-stack and operational controls pass, install it as the Cargo test and ordinary
  verification default and remove the temporary profile. Until then current verifier defaults
  stay release. Optimized production acceptance, required oracle/extension steps and `just qualify`
  remain release-resolved. One selected profile owns commands, artifact paths, receipts and reruns.
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
- Provider source attribution follows the owning crate’s complete relevant normal/build dependency
  and declared asset closure, including conservative target/optional/patch roots. Dev-only and
  unrelated downstream source roots are excluded. Root manifest/lockfile sensitivity remains
  conservative; Cargo owns resolution. The compiler owns native execution composition at
  `Workspace::output`: combine stage implementation and the rooted compiler source digest in
  existing `ContributionSpec.implementation`. Raw provider identity, supplier bindings and
  `Stage.code` remain separate. Imported completed specifications retain captured hashes; this
  changes no semantic compatibility rule and needs no wire column or format bump solely for it.
- Typed adapters retain record, declaration and coverage checks. Shared native task/registration/
  stream mechanics operate after admission; submitted writes own complete charged batches until
  terminality. No per-row erasure, borrowing compatibility path or new executor is selected.
- Coherent extraction/compiler topic groups compile each substantial driver once per binary.
  Discovery, selected module scope and independent fixture ownership remain unchanged.
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

**Historical focused evidence retained from ADR-0136, 2026-10-08:** the pinned compiler compiled and ran a small program with automatic workers;
fixture and build-helper Python controls passed; an actual owned server started with the selected
16 GiB allocation and 8 GiB threshold. These controls do not establish the ongoing persisted
execution plan's product acceptance. The [correction plan](../plans/persisted-execution-corrections-plan_2026-10-07.md)
retains that separate scope and its current verification obligations.

**Accepted target, implementation Proposed, 2026-10-08:** BC0 records both confirmed rule changes. The plan owns normal-stack/profile, charged cancellation/drain, source-closure and cold-transport acceptance. Numerical build/runtime benefits remain unmeasured. A failed candidate is repaired before default installation; production regressions reopen the physical boundary.
