---
name: handoff
description: Rewrite STATUS.md from the actual state of the tree so the next session starts without re-deriving anything. Use at the end of a session that changed what is true, after a slice lands, or when STATUS.md has drifted.
---

# Handoff

Rewrite `STATUS.md` (≤60 lines) from what is true **now**, not from what the session intended.

Gather:

- `git log --oneline -10` and `git status --porcelain` — what landed, what is uncommitted.
- `just check` (or `just test-all` if the session touched extraction, Delta or fixtures) — report
  each outcome as `passed`/`failed`/`blocked`/`not_run` with the command.
- `just adr index` then `docs/adr/README.md` — active decisions; note any still `proposed`.
- `just adr revisit` — triggers that fired.
- The latest review in `docs/design_review/reviews/` and its Deferred rows, if any.

Write:

1. **Increment and slice** — where we are in DESIGN §1.2, and the slice in flight.
2. **Last verified** — command, date, outcome.
3. **Known failures and blocks** — each with the specific fix or prerequisite. Never drop one
   because it is inconvenient.
4. **Open decisions** — proposed ADRs, fired revisit triggers, deferred review findings.
5. **Next** — the single next concrete step.

Date every verification claim. `STATUS.md` is a handoff, not a changelog (git is the changelog)
and not the design (that is DESIGN.md). If they disagree, the tree settles it and both are fixed.
