---
id: ADR-0125
title: Dependencies float; exact pins need a recorded reason
status: superseded
date: 2026-10-04
supersedes: []
superseded-by: ADR-0132
design: [§7]
evidence: Tested
revisit: $ just deps
---

## Context

The operator approved a cross-repository dependency policy on 2026-10-04: agents add libraries
the work warrants without asking, the resolver picks versions, and a pin needs an overt reason
specific to the dependency. This repository pinned by default instead. Every
`[workspace.dependencies]` entry carried `=x.y.z` and a `docs/pins.md` row, which
`scripts/check_family.py::unpinned` enforced (the H1 review F8 rule). The Python members pinned
`==`. Most of those pins had no reason beyond "the version already in `Cargo.lock`".

Two accepted records carry clauses that read as blanket pinning:
- [ADR-0118](0118-pinned-compute-and-analyzer-families.md) (Decision) says "Pin changes go through
  the `pin-check` skill";
- [ADR-0079](0079-shared-cargo-builds.md) (Decision) says "Cargo.lock and root exact pins retain
  version ownership". Its point is that Hakari's generated crate never owns versions.

The responsible owners are DESIGN §7, `docs/pins.md`, `scripts/check_family.py`, the
`pin-check` process skill and the `just deps` / `just upgrade` recipes.

## Options

1. **Keep exact-by-default pins.** Reproducibility already comes from the committed lockfiles and
   the `--locked` gates. Exact manifests add a row and a review for every addition, and they
   block `cargo update`/`uv lock --upgrade` from ever moving a dependency.
2. **Float everything, including families and forks.** This is simplest, but it reopens §B9:
   two Arrows or DataFusions compile and then fail at every boundary. It would also detach the
   Pyrefly/Ruff/ty forks from their reviewed revisions.
3. **Float by default, and keep a pin only for a recorded, dependency-specific reason** (chosen).
   Families, forks and vendored sources, parity oracles, byte-stable or golden outputs,
   content-addressed acquisitions and platform wheels keep their pins and rows.

## Decision

- Manifests use the tool's default specifier: `cargo add` writes a caret and `uv add` a `>=`
  floor. `Cargo.lock` and `uv.lock` own resolved versions. `just upgrade` (root) moves them at an
  agent's discretion. `just upgrade <dir> …` moves named sub-projects; the analyzed libraries
  (`libraries/<name>`) and `services/vllm` do not move through it routinely.
- An exact version, cap, git rev or hold-back needs a `docs/pins.md` row with its reason and
  when to revisit it. `check_family.py::unpinned` is inverted: a caret passes, and an exact pin
  or rev without a row fails. Family single-version and fork checks are unchanged.
- `just deps` runs cargo-deny `bans sources` without `licenses`. `unknown-git = "allow"`,
  because a git dependency is an exact pin and already needs a row. The named fork allowlist
  stays as documentation.
- Narrowing by reference, without editing either record:
  - ADR-0118's "Pin changes go through the `pin-check` skill" applies to deliberate pins: its
    families and forks, and every other row in `docs/pins.md`.
  - ADR-0079's "root exact pins" means the reasoned exact pins. Cargo.lock owns every other
    version.
  - Neither decision changes: Hakari still owns no version, and the families and forks keep
    their pins. This record therefore narrows both and supersedes neither.
- Toolchains (`rust-toolchain.toml`, `.python-version`), CI action SHAs and content-addressed
  acquisitions are outside this change.

DESIGN §7 is amended in the same change.

## Consequences

- Adding a crate or Python package needs no ADR, no row and no review of its version.
- Upgrades become one recipe plus the tests the move affects.
- Exact pins are now rarer, and each one carries its reason.
- A transferred library-skill claim is checked against the locked version.
- A version bump may re-key derived artifacts; that is recorded, not refused.

Implementation and verification on 2026-10-04:
- 38 Rust entries floated to carets with `Cargo.lock` unchanged
  (`cargo metadata --locked`).
- Python floors re-locked with metadata-only `uv.lock` changes.
- The inverted check is covered by `tests/scripts/test_check_family.py`.

Integrated `just test-all`/`just hygiene` results are reported with the change. Revisit if a
floated dependency breaks a contract between upgrades. Such a break is a recorded reason to pin
that dependency, not a reason to restore blanket pins.
