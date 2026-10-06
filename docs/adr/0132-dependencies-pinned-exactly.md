---
id: ADR-0132
title: Declared dependencies are pinned exactly
status: accepted
date: 2026-10-06
supersedes: [ADR-0125]
superseded-by: null
design: [§7]
evidence: Tested
revisit: $ just deps
---

## Context

[ADR-0125](0125-dependencies-float.md) let manifests float (carets and `>=` floors) and moved
lockfiles with `just upgrade` at an agent's discretion. In practice versions shifted under agents
working in parallel, environments were disrupted, and agents were surprised by versions they had
not chosen. The operator reversed the cross-repository policy on 2026-10-06. The owners are
DESIGN §7, `docs/pins.md`, `scripts/check_family.py`, the `pin-check` skill and `just deps`.

## Options

1. **Keep floating manifests.** The lockfile still pins, but every routine re-resolve moves
   everything, and nothing in a manifest tells an agent which version it is working against.
2. **Pin exactly with a reviewed process for every bump.** Stable, but it brings back the ceremony
   (rows, approvals) that made adding a library expensive.
3. **Pin every declared dependency exactly; changing a version stays agent judgment** (chosen).

## Decision

- Every declared dependency is exact: Rust `=x.y.z` (root `[workspace.dependencies]` and member
  manifests), Python `==x.y.z` (root, `python/lctx_mcp`, `services/vllm`; `[tool.uv]
  add-bounds = "exact"` makes `uv add` write it). A git dependency is pinned by its `rev`. The
  committed lockfiles hold everything beneath.
- Adding or bumping a dependency needs no ADR, approval or pins row: move that dependency (or its
  family), check the lock diff, run the affected tests and name the move in the commit. No
  wholesale re-resolve (`uv lock --upgrade`, bare `cargo update`) unless the operator asks;
  `just upgrade` is removed.
- `docs/pins.md` lists holds only: versions not to bump casually, each with reason and revisit
  trigger. `check_family.py::unpinned` now fails a caret or range on a registry dependency or a
  git source without `rev`, and passes an exact pin without a row. Family and fork checks are
  unchanged. The cargo-hakari generated crate is not a declared manifest and is not checked.
- ADR-0118's families and forks, ADR-0079's toolchain and the analysed `libraries/*` keep their
  own rules.

DESIGN §7 is amended in the same change.

## Consequences

- An environment changes only when someone changes a version on purpose.
- Bumps are one deliberate move each; the commit names it.

Implementation on 2026-10-06: 38 Rust requirements and 8 Python requirements were frozen at their
locked versions. `Cargo.lock` was unchanged (`cargo metadata --locked`), the `uv.lock` diffs moved
specifier metadata only (`uv lock --check`), and `tests/scripts/test_check_family.py` passed.
