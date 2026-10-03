---
id: ADR-0119
title: Declare the actual extraction provider per fact family
status: accepted
date: 2026-10-03
supersedes: []
superseded-by: null
design: [§B8, §4.2, §15.11]
evidence: Interface-checked
revisit: Independent providers need different retry or execution lifetimes within one scheduled extraction stage.
---

## Context

ADR-0117 introduces canonical latest Ruff alongside Pyrefly typing. Their narrow adapters currently
run together in one captured-input session; each support names its actual provider and run.
The stage contract previously named one reporting provider for every family, so frontier admission
rejected Ruff Syntax coverage as unexpected. The [coordinator](../plans/code-facts-expansion-plan_2026-10-03.md)
owns implementation and qualification; the initial focused run demonstrated that rejection.

## Options

1. **Separate scheduled Ruff and Pyrefly stages.** Appropriate for independent retry/lifetime,
   but currently requires an additional shared parsed-tree handoff or another canonical parse.
   It does not improve ownership of the model relations in this joint captured-input session.
2. **Attribute all coverage to Pyrefly or a composite provider.** Smaller scheduling change,
   but hides the actual syntax supplier or invents a composite semantic authority; rejected.
3. **Explicit family/provider grants in the existing stage** (chosen). Coverage identity follows
   actual source identity while the stage retains one writer per output and one outcome.

## Decision

`Stage.coverage` declares finite `FamilyCoverage { family, provider }` pairs. Remove the redundant
single `Stage.provider` field. Frontier admission derives its exact expected scope/family/provider
rows from those grants and reconciles one stage outcome over all its declared pairs. Duplicate
pairs are invalid; another provider's rows are not admitted merely because the family matches.
Stage identity includes both members of every grant in canonical order. This is scheduled
coverage metadata, not a provider discovery registry or an alternative assertion authority.

## Consequences

Ruff Syntax and Pyrefly typing can retain independent provider/run evidence in one extraction
session. Empty-coverage pure stages name no provider. Existing single-provider stages declare
one grant per family; their relation ownership, budgets and effects are unchanged. Schema/source
and wrong-provider controls remain required before publication. A future independent execution
lifetime should split stages rather than extending this contract into a scheduler registry.
Current adoption and targeted controls are in progress; full-series acceptance remains open.
