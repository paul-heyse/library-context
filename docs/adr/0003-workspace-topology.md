---
id: ADR-0003
title: Core workspace plus separate adapter workspaces and processes
status: superseded
date: 2026-09-22
supersedes: []
superseded-by: ADR-0006
design: [§B8]
evidence: Proposed
revisit: The first adapter builds against the core family without conflicts, and the process boundary costs more than it isolates.
---

## Context

Ruff's library crates (0.0.x line) and Pyrefly's crates (unpublished, git only) move fast and
bring their own dependency graphs. The core needs exactly one Arrow/DataFusion/delta-rs family
(ADR-0002). Initial_plan §5 recommends two separate adapter processes emitting Arrow, so that
analyzer internals are isolated from the core's dependency graph.

## Options

1. **The simpler alternative: one Cargo workspace linking Ruff, Pyrefly and the core.** One
   build and no IPC. But any shared transitive dependency must agree across three fast-moving
   projects, and a Pyrefly bump could force an Arrow bump.
2. **Separate workspaces and processes, joined by Arrow IPC files** (chosen, provisionally).
3. **Separate workspaces joined by JSON.** Rejected: it violates §B2 and needs schema inference.

## Decision

- **Core workspace** at the repo root: `crates/*`, starting with `cpg-schema`.
- **One adapter workspace per analyzer**: `adapters/ruff-extract/` and
  `adapters/pyrefly-extract/`, each with its own `Cargo.lock` and, if needed,
  `rust-toolchain.toml`.
- Adapters depend on only `arrow-array`/`arrow-schema`/`arrow-ipc` at the family version, plus a
  path dependency on `cpg-schema` for the table contracts, so the schema has one authority.
- `just test-all` and `just deps` iterate over the adapter workspaces.

The status stays `proposed` until the first adapter exists and proves whether `cpg-schema` links
cleanly next to the analyzer's dependencies.

## Consequences

Two extra builds and lockfiles, plus a process boundary with IPC files. Adapter crashes and
analyzer panics are contained, and each analyzer's revision can move independently. The Ruff and
Pyrefly revision choice is a separate decision (increment 1).
