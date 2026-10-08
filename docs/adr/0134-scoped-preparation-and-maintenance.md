---
id: ADR-0134
title: Verification readiness observes, preparation is explicit and scoped, and end-of-turn maintenance may be scoped to a turn's paths
status: accepted
date: 2026-10-07
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Interface-checked
revisit: An agent workflow needs readiness to prepare implicitly again, or scoped maintenance leaves generated outputs stale after the dirty work it protected is committed.
---

## Context

[ADR-0126](0126-contract-assurance-pivot.md) states that verification readiness "prepares only declared Python member/group closure, once before execution". Its launcher does this by running `uv sync` inside `verify.py::prepare`. The same ADR has the root agent run whole-tree `just turn-end` at the end of every turn that changed files.

The [agent workspace effectiveness assessment](../plans/agent-workspace-effectiveness-plan_2026-10-07.md) found that both clauses cause friction for agents:

- **AE-01:** `just` recipes carry the native extension's uv cache key and bare launches do not, so each syncing invocation rebuilds the 175 MB `lctx_semantics` extension in turn.
- **AE-02:** readiness runs `uv sync` inside the shared environment while other agents' native workers may be live, contrary to AGENTS.md's own rule.
- **AE-21:** whole-tree `turn-end` reformats and regenerates another agent's uncommitted work. Two documentation-only turns on 2026-10-07 left it `not_run` for that reason.

The [capability review](../design_review/reviews/design_review_agent-workspace-effectiveness-capabilities_2026-10-07.md) (F03, F04, F08) asked for preparation scoped by project and requirement, managed environment ownership, and scoped maintenance. The operator approved this route on 2026-10-07 (plan §5.9, OD1).

ADR-0126's other decisions are unchanged and not restated here: trusting acknowledged immutable inputs, explicit verification families, and assembled qualification at its boundary.

## Options

1. **Keep readiness preparing, but scope it to each requirement.** This is the simplest change. It stops the native rebuild from triggering when only tools are needed. But a verification run still mutates the shared environment under live importers, and one command keeps two jobs, observing and repairing, so a blocked result cannot name an independent repair.
2. **Readiness observes; preparation is a named, explicit route; synchronization takes exclusive managed ownership.** *(Selected.)*
   - Each boundary checks only what it imports. The native check uses `uv sync --locked --check --inexact` with the native key.
   - A failed check reports `blocked` together with the route that repairs it: `just sync tools|native|vllm`, which `just ready` composes.
   - Managed commands hold shared ownership of the environment and of the checkout's extension directory. Synchronization takes the same resources exclusively and reports who holds them.
   - It costs one explicit step after an input change. In exchange, synchronization happens once and visibly, never as a side effect of a test run.
3. **Lock the whole repository, or forbid concurrent verification.** Rejected. It serializes unrelated pure-Rust work, and fd-inheriting locks have caused deadlocks before.

For maintenance, the alternatives are:
- **Keep whole-tree `turn-end` mandatory.** Agents must then either rewrite another agent's dirty files or skip formatting and generators altogether.
- **Allow `turn-end` scoped to the turn's paths, with generators run only when their inputs are in scope.** *(Selected.)* The cost: a step skipped because of the scope must be reported and done later as a whole-tree run.

## Decision

- **Readiness observes.** Verification readiness checks each boundary's prerequisites and reports `blocked` with the named repair route. It never synchronizes an environment.
- **Preparation is explicit and scoped.** It runs only through the explicit routes: `just sync tools` (the dev group only, no extension build), `just sync native` (the full locked sync, with the native key computed then) and `just sync vllm` (the separately locked service). `just ready` composes them without repeating a synchronization.
- **No implicit sync.** Every launcher the repository defines passes `--no-sync` or `--no-project`.
- **Managed ownership.**
  - Managed synchronization takes exclusive ownership of the selected environment and the checkout's extension directory, and reports live shared holders while it waits.
  - Managed consumers hold shared ownership for their lifetime. Pure-Rust work holds none.
  - Unmanaged commands remain available, outside this guarantee.
- **Scoped maintenance.** When whole-tree `just turn-end` would rewrite another agent's uncommitted work, the root agent may run `just turn-end --paths …` over the paths its turn changed. Generators run only when their inputs are in scope. The turn's report names any step that was not performed. Whole-tree `turn-end` remains the default.

DESIGN §1.2 records these rules.

> Decision: ADR-0134

## Consequences

**Easier:**
- An environment changes only through an explicit step that names its route.
- Rebuild ping-pong between `just` and bare invocations ends.
- A blocked verification names its repair.
- A turn can complete its own maintenance without touching another agent's work.

**Harder:**
- After a native input changes, an agent runs `just sync native` before the native boundaries.
- Scoped maintenance can leave a generated output stale until the next whole-tree run. The freshness check reports such outputs.

This record is accepted while implementation is in progress. Packets P2 (preparation and ownership) and P6 (scoped maintenance) of the [plan](../plans/agent-workspace-effectiveness-plan_2026-10-07.md) implement it. The plan's §9 owns its findings and verification. AGENTS.md's testing and turn-end wording changes when that behaviour lands (P7).

ADR-0126 remains accepted. A dated pointer under its `## Amendments` names the two clauses this record governs. Full supersession of ADR-0126 will consolidate its surviving decisions into one replacement, once the paused product edits to `docs/design/sections/semantic-model.md` are committed.
