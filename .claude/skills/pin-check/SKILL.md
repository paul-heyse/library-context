---
name: pin-check
description: Add, bump, hold or release a dependency version, or check a claim about which version is in use. Use before changing a version in a manifest or in docs/pins.md.
---

# Pin check

Every declared dependency is pinned exactly and the lockfiles hold the rest (ADR-0132).

- **Add or bump:** `uv add 'pkg==x.y.z'` (plus `--project services/vllm` for the service), or
  `cargo add pkg@=x.y.z` (root `[workspace.dependencies]` or the member manifest). Move a family
  together (Arrow/DataFusion/object_store, salsa; DESIGN §7 / ADR-0118). Read the lock diff, run the
  affected tests and `just deps`, and name the move in the commit. Glance at the library-skill delta
  rows in `docs/pins.md`. Never re-resolve wholesale unless the operator asks.
- **Forks:** a Pyrefly or Ruff/ty fork change gets a new pinned revision and patch; run
  `scripts/check_pyrefly_fork.py` / `scripts/check_ruff_fork.py` and update the revisions declared in
  `scripts/check_family.py`. Moving a family or fork pin supersedes ADR-0118; the toolchain is ADR-0079.
- **Hold:** add a `docs/pins.md` row with the reason, revisit-when and a dated verification.
  **Release:** delete the row.
- **Version claims:** read them from the lockfile or installed metadata in this session, and date them.
  A skill's version that differs from the locked one leaves a transferred claim unverified: say so.
