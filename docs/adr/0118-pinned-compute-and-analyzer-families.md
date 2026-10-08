---
id: ADR-0118
title: Pin compute once and scope the independent and Pyrefly-embedded analyzer families by source identity
status: accepted
date: 2026-10-03
supersedes: [ADR-0090]
superseded-by: null
design: [§B9, §7]
evidence: Interface-checked
revisit: $ just deps
---

## Context

ADR-0117 selects independent latest Ruff while Pyrefly keeps its embedded version. Version-only
family checks cannot distinguish equal-version crates from different sources. This record carries
forward ADR-0090's compute and PostgreSQL-provider decisions and replaces its analyzer exception.
`docs/pins.md` owns current exact revisions; the [code-facts coordinator](../plans/code-facts-expansion-plan_2026-10-03.md)
owns implementation and acceptance. Compute pins are unchanged.

## Options

1. **Retain a single Ruff version.** Simple, but reimposes the rejected embedded-version ceiling.
2. **Permit arbitrary duplicates.** Avoids policy work, but silently admits incompatible ASTs,
   equal-version mixed-source types and feature leakage.
3. **Scope two exact analyzer families by version and nominal Cargo source** (chosen). One
   producer configuration owns each source identity and attachment is explicit.

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
  no type crosses their boundary. The analyzer scopes below replace the former `cpg-flow`-only extra family.
- Pin changes go through the `pin-check` skill.

The toolchain pin itself belongs to ADR-0079.

**Analyzer scopes (accepted target).** Independent Ruff/ty crates are one exact fork revision
at the reviewed latest release, reachable by `cpg-extract` and `cpg-flow`. Most crates are 0.0.16;
`ruff_linter` is 0.16.10. Pyrefly 1.4.0-dev.3 and its single aggregate patch retain registry Ruff
0.0.14 strictly inside the Pyrefly adapter. No native AST/model type crosses that boundary.
The one ty graph pins salsa, salsa-macros and salsa-macro-rules exactly to 0.28.5; embedded Ruff's
salsa feature stays off. Hakari must not unify features across these nominal families.

Family checks resolve dependencies by name, version and Cargo source, reject ambiguous bare
references, and permit duplicates only in these declared scopes. Fork checks verify immutable
parent/tag/patch digests and classify environment reads. Cargo-deny's allowlist names the owned
Ruff fork in addition to existing technical choices; licenses are not a rejection reason.

## Consequences

**Interface-checked, 2026-10-03:** published/tag manifests and nominal family boundaries were read.
Production lock migration, native adapter parity and policy controls are M1; full `just deps`
inside hygiene and `just test-all` remain Q0 obligations. The existing compute family keeps its
prior dated Tested receipts. This decision does not claim latest-provider product acceptance.
The owned PostgreSQL provider continues bounded generation reads/pools/chunks without federation;
its changing exact revision belongs in `docs/pins.md`. A future lake/federation or additional
production type authority is a new decision.

## Amendments

- 2026-10-08: ADR-0136 now owns the surviving toolchain/shared-build clauses of retired ADR-0079.
