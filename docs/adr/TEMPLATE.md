---
id: ADR-NNNN
title: TITLE
status: proposed
date: YYYY-MM-DD
supersedes: []
superseded-by: null
design: []
evidence: Proposed
---

## Context

What forces this decision now; the expected change and affected owners/consumer contracts.
Cite stable architectural section IDs and direct owner links and source findings where they carry the argument. Identify current
implementation separately from the proposed target.

## Options

1. **The simplest viable alternative** — say why it loses or wins.
2. Other meaningful choices. Compare change propagation, semantic ownership, composition,
   isolated testing and total library/bespoke integration burden where relevant.

## Decision

State the selected responsibility/contract/mechanism so a future session can assess it.
Amend governed sections in DESIGN or the focused architectural collection in the same commit, ending with `> Decision: ADR-NNNN`.

## Consequences

What becomes easier and harder; scope limits and the event that would reopen the decision
(add `revisit:` metadata, with `$ command` only when a command safely decides it).
Acceptance records a decision in force. State remaining implementation and verification and
link to the plan's current disposition owner; do not mark a finding closed solely because this
record is accepted.
