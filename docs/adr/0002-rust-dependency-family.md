---
id: ADR-0002
title: Pin DataFusion 55.1 / Arrow 59.3 / delta-rs git 58f07cd6 as one family
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§B9, §7]
evidence: Tested
revisit: $ just deps
---

## Context

Initial_plan §7.4 found that the DataFusion 55.0.0 tag uses Arrow 59.2, DataFusion main uses
Arrow 60, and the delta-rs workspace uses Arrow 59 / DataFusion 55. The published `deltalake`
crate is not compatible with DataFusion 55 and Arrow 59. Two Arrow versions in one graph compile,
but their types don't interoperate at any boundary. The `deltalake` capability skill was built
against delta-rs git `58f07cd6` with DataFusion 55.1.0, Arrow 59.3.0 and object_store 0.13.2,
and its probes ran against that profile.

## Options

1. **The simpler alternative: published `deltalake` with whatever DataFusion it pulls in.**
   Rejected. It doesn't match the DataFusion 55 / Arrow 59 line, and we would lose the skill's
   evidence, which is pinned to the git profile.
2. **Track delta-rs `main`.** Rejected. The API drifts under us and the skill's evidence stops
   applying.
3. **Pin the skill's exact profile** (chosen).

## Decision

`[workspace.dependencies]` pins:
- `datafusion =55.1.0` (`sql`, `parquet`, no default features)
- `arrow-* =59.3.0`, `parquet =59.3.0`
- `object_store =0.13.2`
- `petgraph =0.8.3`
- `deltalake` from git rev `58f07cd62bfbce3649a7e1c87c696288068ae184`, with `datafusion` and
  `rustls`

`rustls` is needed because `deltalake-core` refuses to compile without one TLS feature, even for
local-only use. The kernel (`buoyant_kernel` 0.25.1, branch `buoyant/main`) is pinned only by
`Cargo.lock` at `8ba063f8f84fec222000f66d40d70911d7c79675`, which matches the skill. A bare
`cargo update` could move it, so `Cargo.lock` is committed and pin changes go through the
`pin-check` skill. The toolchain is pinned to 1.98.1; the highest MSRV in the family is
delta-rs's 1.94.1.

## Consequences

**Evidence (Tested, 2026-09-22):**
- `cargo nextest run`: `family_smoke` writes two Delta commits to a tempdir and queries them
  through the Delta table provider with DataFusion SQL. Both tests passed.
- `scripts/check_family.py`: every family crate resolves to exactly one version, and a
  synthesized second `arrow-array` fails it.
- cargo-deny 0.20.2 does **not** see duplicates that enter only through workspace
  dev-dependencies. That is why the version rule lives in `check_family.py` and not in
  `deny.toml`.

We did not need the vendored DataFusion/kernel patches the predecessor repository carried for
native replay. If we need them, that is a new ADR.

## Amendments

- 2026-09-23: the library-leverage review (O5) corrects the reason `check_family.py` exists.
  cargo-deny 0.20.2 does see dev-only duplicates (`multiple-versions-include-dev`), but it needs
  each crate named, and the family is 78 crates with sub-crates an upgrade can add; the script's
  patterns cover them. Also (O2): a `[workspace.dependencies]` entry no crate uses pins nothing,
  so `just deps` runs `cargo shear`. `object_store` is held by `Cargo.lock` and the family check,
  and `parquet` became a real direct dependency (zstd writes).
