---
name: adr
description: Record, supersede or check an architecture decision. Use when a change alters a DESIGN.md §B binding decision, chooses between real alternatives, or would surprise a future session; when pivoting away from an earlier decision; or when asked why the design is the way it is.
---

# ADR

`docs/design/DESIGN.md` says what the system **is**; `docs/adr/` says **why** and what was
rejected. `docs/adr/README.md` (generated) lists active decisions and supersession chains —
read it first when asked "why".

## When

| Change | Record |
|---|---|
| Alters a §B binding decision | ADR + a `standard` design review (ADR-0001) |
| Chooses between real alternatives, or would surprise a future session | ADR |
| Pivots away from an accepted decision | `just adr supersede ADR-NNNN <slug>` — never edit the old record |
| Bug fix, refactor inside a declared contract, new tests | none |

## How

1. `just adr new <slug> --title "Decision stated as a sentence"` (or `just adr supersede …`).
2. Fill Context → Options → Decision → Consequences, one page. **Options always include the
   simpler alternative**, and say why it loses (or that it wins).
3. `design:` lists the DESIGN.md `§` sections it governs. Amend those sections **in the same
   commit** and end each with `> Decision: ADR-NNNN`; add a Revision history row. Never
   renumber sections — insert `§3.2.1`.
4. `evidence:` is a charter §D label at the strength actually established (`Proposed` until
   something ran). `revisit:` is the trigger to reconsider; prefix `$ ` if a command decides it.
5. `status: proposed` while unproven; `accepted` once the decision is in force. Accepted records
   are immutable except `status`/`superseded-by` — `just adr lint` diffs them against `HEAD`.
6. `just adr index`, then `just adr lint`.

`just adr revisit` lists every active trigger and runs the runnable ones.
