---
id: ADR-0090
title: Pin DataFusion 55.1 / Arrow 59.3 as one family, without delta-rs or federation
status: accepted
date: 2026-09-29
supersedes: [ADR-0002]
superseded-by: null
design: [§B9, §7]
evidence: Tested
revisit: $ just deps
---

## Context

ADR-0002 pinned DataFusion 55.1, Arrow 59.3 and delta-rs git `58f07cd6` as one family, because
the published `deltalake` crate did not match that line and the Delta store needed its table
provider. ADR-0086 replaces Delta with immutable PostgreSQL generations. Cutover plan P1.3 removed
the Delta runtime, so delta-rs, its kernel (`buoyant_kernel`) and the Delta log filters have no
remaining consumer.

`datafusion-federation` entered with the old PostgreSQL reader (`postgres_read`, removed at P1.2)
through the owned table-provider fork. The generation-bound provider sessions of P1.10 read one
pinned generation directly and need no federation (plan T13).

## Options

1. **Keep ADR-0002 unchanged.** This is the simplest choice, and it loses: an unused git
   dependency and its kernel stay in every build and in the deny policy. The family rule would
   keep guarding a crate no code uses.
2. **Drop delta-rs but keep federation in the fork's defaults.** This loses because it keeps an
   unused query-planning layer in the provider's dependency closure, against T13.
3. **Drop both, and make federation opt-in in the fork** (chosen). The fork's workspace entry for
   `datafusion-table-providers-common` disables its defaults, so federation reaches a crate only
   through that crate's own `federation` feature.

## Decision

`[workspace.dependencies]` pins:
- `datafusion =55.1.0` (`sql`, `parquet`, no default features);
- `arrow-* =59.3.0` and `parquet =59.3.0`;
- `object_store 0.13.2`, held by `Cargo.lock`;
- `petgraph =0.8.3`;
- the owned `datafusion-table-providers-postgres` fork at rev `790726de`, defaults off and
  without federation.

There is no delta-rs, no `buoyant_kernel` and no `datafusion-federation`.

These clauses of ADR-0002 and its amendments are restated and remain in force:
- Every family crate resolves to exactly one version in the core workspace.
- `scripts/check_family.py` enforces the single version, because cargo-deny needs each crate
  named and the family's sub-crates change with upgrades.
- A `[workspace.dependencies]` entry no crate uses pins nothing, so `just deps` runs
  `cargo shear`.
- Extra families are allowed only when declared in `scripts/check_family.py` with their scope, and
  no type crosses their boundary. The one in use is the `cpg-flow` ruff/ty 0.0.14 line with salsa
  held at exactly 0.28.2.
- Pin changes go through the `pin-check` skill.

The toolchain pin itself belongs to ADR-0079.

## Consequences

- Evidence (Tested, 2026-09-29):
  - `family_smoke` queries Arrow batches through DataFusion SQL;
  - `scripts/check_family.py Cargo.lock` passes and its tests pass;
  - `Cargo.lock` holds none of `deltalake`, `buoyant_kernel` or `datafusion-federation`;
  - the fork's postgres crate checks with `--no-default-features`.
- `just deps` and `just test-all` run at plan Q, with the rest of the P0–P2 integrated gates.
- The `deltalake` library skill is no longer selected, and the Delta-specific ast-grep rules
  (`delta-write-path`, `no-raw-parquet-scan`) are removed.
- A future reason to read Delta or Parquet lakes directly would be a new decision.
- The table-provider fork is unused in the workspace until P1.10 adds generation-bound sessions.
  P1.9 adds bounded chunks to the same fork.
