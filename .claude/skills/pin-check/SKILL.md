---
name: pin-check
description: Change or re-verify a dependency or tool pin (Rust family, delta-rs/kernel git revs, ruff/pyrefly revisions, toolchain, Python dev tools). Use before editing any version in Cargo.toml, rust-toolchain.toml, pyproject.toml or .python-version, and when a claim about "what version we use" needs checking.
---

# Pin check

Pins are recorded in `docs/pins.md`; the Rust family's authority is DESIGN §7 / ADR-0002.

1. **Read the primary source in this session** — the crate's `Cargo.toml` in
   `~/.cargo/registry/src` or `~/.cargo/git/checkouts`, the upstream tag, or the tool's own
   `--version`. Context7 and memory are leads, not evidence. Never report a version you did not
   read in this session.
2. Check the matching library skill's pinned profile (`.claude/skills/<library>/SKILL.md`). If
   the new pin leaves that profile, say so: the skill's evidence no longer transfers.
3. Change the pin, then run `just deps` (single-version family check + cargo-deny) and
   `just test-all`. Use `cargo update -p <crate>` — never a bare `cargo update`, which can move
   the git-branch kernel pin silently.
4. Add or update the row in `docs/pins.md` with today's date and the command that verified it.
5. If a Rust-family or analyzer pin moved, that is an ADR (`just adr supersede ADR-0002 …` for
   the family). A dev-tool bump needs only the pins row.

A contradiction between a documented claim and what you read is a finding: stop and report it.
