---
name: pin-check
description: Deliberately pin, cap or hold back a dependency or tool, remove such a pin, or verify a claim about which version is in use. Not needed to add or upgrade a dependency — those float (`uv add`, `cargo add`, `just upgrade`). Use before writing an exact version, upper cap or git rev into Cargo.toml, pyproject.toml or docs/pins.md, before moving a family, fork or toolchain pin, and when a "what version do we use" claim needs checking.
---

# Pin check

Dependencies float by default (ADR-0125): adding one or moving to the latest needs no pin and no
row. This skill covers the deliberate exceptions recorded in `docs/pins.md`: the compute and
analyzer families and forks (DESIGN §7 / ADR-0118), the toolchain (ADR-0079), and every other
exact version, cap, git rev or hold-back with its reason. Documentation binaries are listed in
`docs/site.toml`; one given a version there is a pin and needs a row too.

1. **Read the primary source in this session**: the lockfile (`uv.lock`, `Cargo.lock`), the
   package's own manifest (for a crate, its `Cargo.toml` in `~/.cargo/registry/src` or
   `~/.cargo/git/checkouts`), the upstream tag, or the tool's own `--version`. Context7 and
   memory are leads, not evidence. Never report a version you did not read in this session.
2. **Pin only for an overt reason specific to the dependency:**
   - a named breakage or incompatibility;
   - a type-sharing family that must resolve to one version;
   - a fork, git rev or vendored source;
   - a content-addressed acquisition;
   - golden or byte-stable output, or a private-API use;
   - a parity or reference oracle;
   - an experimental control;
   - platform or wheel availability.

   Not reasons: reproducibility (the lockfile gives it), "already in the lock", or a version
   entering a key or digest (a bump re-keys).
3. Write the pin, then run the tests the change affects; `just test-all` at the end of the
   scope. `just hygiene` runs `just deps`: every exact pin or git rev has a row, declared
   families resolve to one version (`scripts/check_family.py`), cargo-deny checks bans and
   sources, and the fork checks run.
4. **Fork revisions.** A Pyrefly or Ruff/ty fork change gets a new pinned revision and patch:
   `uv run python scripts/check_pyrefly_fork.py` and `uv run python scripts/check_ruff_fork.py`
   verify the immutable parent/tag, the patch digest and classified environment reads. Update
   the revisions declared in `scripts/check_family.py` with the fork.
5. Add or update the row in `docs/pins.md`: the pin, its reason, when to revisit it, today's
   date and the command that verified it. Removing a pin removes its row; the lockfile then
   holds the version.
6. If a §B decision governs a pin, moving it is an ADR: a compute-family pin (§B9) or an
   analyzer family or fork pin supersedes ADR-0118 (`just adr supersede ADR-0118 …`); the
   toolchain belongs to ADR-0079. A dev-tool bump needs only its row.

Library skills record the version they were built at, and pins that exist for a skill are
outside this policy for now. When a skill's version differs from the locked one, a transferred
claim is unverified at the locked version: say so.

A contradiction between a documented claim and what you read is a finding: stop and report it.
