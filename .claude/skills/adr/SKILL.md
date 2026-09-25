---
name: adr
description: Record, supersede or check an architectural decision, including ownership, contracts and meaningful alternatives. Use when a change alters DESIGN.md binding decisions, pivots from an accepted decision, or needs durable rationale for a future session.
---

# ADR

DESIGN.md plus `docs/design/sections/` owns accepted architecture and labeled targets; ADRs own why and what was rejected.
Read the generated `docs/adr/README.md` for current decisions and open choices; retired records
are recovered from Git, never read as a chain (ADR-0042).

## When

| Change | Record |
|---|---|
| Alters a §B decision or an important ownership/contract boundary | ADR and design review at the binding's cadence |
| Chooses between meaningful alternatives or would surprise a future session | ADR |
| Pivots away from an accepted decision | `just adr supersede ADR-NNNN <slug>` |
| Refactor inside an accepted contract, bug fix or new tests | No ADR solely for the implementation change |

## How

1. `just adr new <slug> --title "Decision stated as a sentence"` or `just adr supersede …`.
2. Use Context → Options → Decision → Consequences. Identify the expected change, responsible
   components and affected consumer contracts. Include the simplest viable alternative; compare
   coupling, composition, local testing and total integration burden where they distinguish options.
3. `design:` lists stable governed section IDs across the architectural collection. Amend them in the same commit, retaining section
   IDs and a `> Decision: ADR-NNNN` line. Keep the section current rather than appending a
   revision diary. Reference executable contracts rather than duplicating their field definitions.
4. `evidence:` labels the stated claim at its established strength. `status: accepted` means the
   decision is in force; it does not mean all implementation is complete or verified. Distinguish
   those facts in Consequences and the plan's finding disposition. Record the revisit trigger;
   prefix `$ ` only if a command safely decides it.
5. Accepted records are immutable except `status`/`superseded-by`. A factual correction changing
   no decision may be appended as a dated line under `## Amendments`; a decision change needs a
   superseding record. Preserve unrelated decisions when replacing one process or policy clause.
6. Run `just adr index` and `just adr lint`. The binding owns current finding disposition:
   link source review IDs to the active plan; close findings on evidence of the correction,
   not merely acceptance of this record.

## Replace and retire

When a record is replaced, carry each surviving clause to its current owner or the replacement,
and say which clauses are rejected or out of scope. One replacement may consolidate several
predecessors by responsibility; it restates decisions already in force and never accepts an
unapproved target. Mark the predecessors superseded and update every governing reference
(`rg -n 'ADR-NNNN'` outside `docs/adr/`: AGENTS, binding, skills, STATUS, the plan, the gold
freeze); then, once the replacement is accepted, delete them. A `supersedes` entry naming a deleted record is provenance; numbering skips every id
in Git history (`just adr new` refuses a shallow clone). Keep a self-contained record however old;
keep a proposed record only while its choice is live.

`just adr revisit` executes runnable triggers. Inspect them and use it when relevant; a handoff
or process edit does not by itself require running unrelated functional checks.

Start with the architecture map and relevant owner, not the entire collection. A move keeps the
ID, governing references and legacy heading/fragment with a relocation marker and direct owner
link for live sections; a retired section needs no stub and its ID is never reused. The shared resolver drives lint and the website directory. Publication metadata carries no
architectural authority. Internal implementation changes need no new documentation proof packet.
