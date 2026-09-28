---
id: ADR-0075
title: Use incremental workspace builds and cached optimized dependencies
status: accepted
date: 2026-09-28
supersedes: [ADR-0026]
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: Ordinary edit/rebuild latency or dependency cache misses become a material bottleneck.
---

## Context

On 2026-09-28 the operator selected workspace optimization level 2 and imported dependency
optimization level 3, then authorized improvements to build reuse without extensive benchmarks.
The workspace is in active development and its daily tests use the release profile. Incremental
compilation can reuse work within changing workspace crates; sccache can reuse eligible,
non-incremental dependency compilations. Cargo's existing target artifacts remain the first
reuse layer. This changes the build policy in [DESIGN §1.2](../design/DESIGN.md#section-1-2).

## Options

1. Keep incremental compilation disabled everywhere. Workspace libraries remain eligible for
   sccache, but frequent source edits cannot reuse fine-grained incremental work.
2. Enable workspace incremental compilation and keep imported dependencies non-incremental
   (selected). This supports active edits while preserving dependency compiler-cache eligibility.
3. Add a dedicated optimized test profile. Existing profiles already provide the selected
   optimization levels; another profile adds a separate artifact set and command choice.

## Decision

- Root Cargo profiles set workspace O2 with incremental compilation in dev and release.
  Release explicitly retains 16 codegen units. Imported dependencies use O3 without incremental
  compilation, including non-workspace path dependencies. Keep the existing release test recipes.
- Retain stable Rust 1.98.1, 16 Cargo jobs, the default single frontend thread, sccache, and the
  Clang/mold Linux linker route. Preserve target artifacts and stable paths/flags; avoid routine
  `cargo clean`. The operator's shared local sccache configuration now allows 100 GiB.
- Keep ordinary work on `main` in the current tree. Use a separate worktree for truly parallel
  production edits. Isolated benchmark snapshots may live under ignored `build/`.
- Retain the light process from ADR-0026: DESIGN is current authority, ADRs record decisions,
  reviews follow the declared cadence, `just check` is the loop, `just test-all` precedes commits,
  `AGENTS.md` is canonical, and outcomes use `passed`/`failed`/`blocked`/`not_run`.

## Consequences

Workspace incremental artifacts use more disk and those compiler invocations bypass sccache.
Imported libraries retain compiler-cache eligibility; linked outputs can still be uncacheable.
The expected benefit is improved reuse during active development, with no measured speedup claim.
Existing build and cache processes are left running; new settings apply on subsequent invocations
and cache-server starts. No extra profile, activation script or restart procedure is introduced.

Configuration and existing measurement references are updated. Repository builds, tests,
benchmarks and documentation validation are not rerun for this change at the operator's request.
This decision does not advance the product qualification in the
[forward plan](../plans/behavioral-model-forward-plan_2026-09-24.md#30-consolidated-execution).
