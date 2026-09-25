---
id: ADR-0026
title: Use stable cached builds and the main working tree for development
status: accepted
date: 2026-09-24
supersedes: [ADR-0001]
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: Two representative paired runs show a materially faster or less memory-hungry stable configuration, or cache reuse is ineffective in ordinary development.
---

## Context

The operator chose a permanent development configuration after the isolated build screens in
`docs/design_review/evidence/2026-09-24_rust-build-performance/README.md`. The prior process
(ADR-0001, §1.2) sent exploratory spikes to worktrees. In this repository, separate paths and
changing compiler flags disrupt Cargo artifact reuse. The project already pins stable Rust
1.98.1 and uses Clang with mold. The screens did not run an exact 16-job, one-thread stable versus
nightly comparison; a newer nightly failed in a dependency, and the one-thread nightly screen
showed no clear advantage. This decision follows the operator's preference for a simple, stable
development route, not a proven fastest configuration.

## Options

1. **Stay with stable Rust and the current tree, and encode one cache-friendly Cargo default**
   (chosen). It keeps the pinned toolchain and avoids routine cache fragmentation.
2. **Keep per-command environment settings and worktrees for every spike.** This can isolate
   experiments but makes the normal development route harder to reproduce and repeatedly
   changes build paths.
3. **Pivot the default to nightly and parallel rustc frontend threads.** Single-run screens do
   not establish a reliable advantage for the requested 16-job, one-thread setup. Nightly
   2026-09-13 failed to compile `allocative` 0.3.6 in the locked graph.

## Decision

- Keep `rust-toolchain.toml` at stable 1.98.1. In `.cargo/config.toml`, set 16 Cargo jobs,
  `rustc-wrapper = "sccache"`, and `incremental = false`. Stable rustc retains its default
  single frontend thread. Preserve the existing Clang and mold Linux linker route.
- Work on `main` in the current tree for routine edits, reviews and spikes. Use a separate
  worktree when truly parallel agents must edit production code concurrently. Isolated
  benchmark snapshots may live under ignored `build/`.
- Retain ADR-0001's light process otherwise: DESIGN is current authority, ADRs record decisions,
  reviews follow the declared cadence, `just check` is the loop, `just test-all` precedes commits,
  `AGENTS.md` is canonical, and outcomes use `passed`/`failed`/`blocked`/`not_run`.

## Consequences

Direct Cargo commands and `just` recipes share the same defaults. sccache can cache workspace
crates because incremental compilation is disabled; this can lengthen local recompilation after
an edit, so the overall speed of the exact configuration remains unmeasured. sccache must be on
`PATH`. The selected configuration is a development default, not a claim that every cached
compile is faster. A later paired measurement can change the decision without moving the normal
development loop to nightly now.
