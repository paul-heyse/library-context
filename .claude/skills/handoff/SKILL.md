---
name: handoff
description: Update STATUS.md from the actual tree after work changes the project state. Preserve concurrent work, distinguish decisions from verified implementation, and link to the single current owner of finding disposition.
---

# Handoff

Keep STATUS.md within 60 lines. It is a current checkpoint; DESIGN owns architecture, ADRs own
rationale, reviews own dated evidence and the active plan owns scheduled findings' current status.
The binding defines disposition transfer and closure. Link to that owner rather than maintaining
another mutable finding list. Preserve historical receipts with their date and scope.

## Gather

- `git log --oneline -10` and `git status --porcelain`: landed work and concurrent edits.
- Outcomes of checks actually run for this scope. Use the repository's acceptance timing: a
  handoff does not trigger `just check`, `just test-all` or `just pilot`. Report omitted checks
  as `not_run` with the reason; never promote another session's receipt to a current result.
- Current ADR index and relevant decisions. If records changed, run `just adr index` and
  `just adr lint`. Inspect relevant revisit conditions; run commands only when due and within scope.
- Current plan disposition rows and review conclusions. Distinguish the bounded slice judgment,
  enclosing architectural status and remaining integrated acceptance.

## Write

1. **Increment and slice:** functional scope and process changes, committed or dirty.
2. **Last verified:** dated commands and `passed`/`failed`/`blocked`/`not_run`; name missing prerequisites.
3. **Known failures, blocks and decisions:** do not drop an unresolved issue; link to its current
   disposition owner and summarize the consequence without replicating the full register.
4. **Next:** the next concrete action, responsible component and verification boundary.

Read shared files again before writing. Preserve changes made by other work, and do not claim
this session tested them. The tree establishes implemented state; a target remains labeled until
implemented and supported by the appropriate evidence.
