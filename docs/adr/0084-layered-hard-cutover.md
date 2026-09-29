---
id: ADR-0084
title: Pivot through a hard layered cutover with legacy adapters and parity
status: accepted
date: 2026-09-29
supersedes: []
superseded-by: null
design: [§15, §1.2]
evidence: Proposed
revisit: A phase cannot reach exact parity except through undeclared divergences; adapter code for one phase exceeds the new producer code it bridges; or a phase stalls long enough that the operator re-prioritizes product work over completing the cutover.
---

## Context

ADR-0082 and ADR-0083 set a target that changes every layer: the relation vocabulary, identity
recipes, the store and serving. The code is about 105k lines of Rust, with 485 Rust and 204 Python tests
and a working MCP product (PR5, `fedd4a0`). The project is in design phase with one operator and no
production users.

**The operator's direction (2026-09-29):**
- Build the core as new code first.
- Move each layer wholesale.
- Bridge the rest with compatibility adapters.
- Complete a layer only when no legacy code remains in it.
- Avoid a series of blended moves.
- Pause product work until serving cuts over.
- Triage research engines by ablation.
- Define the core contracts first and let them evolve until their layer cuts over.

## Options

1. **Blended incremental consolidation inside the current code.** Lowest risk per step. It leaves long
   hybrid states in every layer, with no clear completion criterion, and the new vocabulary is
   constrained by legacy shapes.
2. **Big-bang rewrite without adapters.** The fastest to write. There is no oracle until the end:
   divergences surface late and cannot be attributed to a layer.
3. **Vertical feature slices through every layer.** Each slice delivers value, but every layer stays
   mixed until the last slice. This is the blended state the operator rejects.
4. **Top-down, serving first.** New serving over old relations would wrap legacy semantics in the new
   contracts before those semantics exist.
5. **Bottom-up hard layered cutover with legacy adapters and parity (chosen).** It follows the
   direction of authority, so each higher layer is rewritten once, onto final identities and relations.

## Decision

**Phases.** Phases run in order. Each exits only when its layer contains no legacy code:

| Phase | Scope | Exit when |
|---|---|---|
| 0 Core | `lctx-model` contracts, the store kernel in `lctx-postgres`, the stage table, and the adapter and parity frameworks, built standalone and tested on the review's known-answer shapes | No dependency on `cpg-schema`/`cpg-core` |
| 1 Store | The existing relations move from Delta to PostgreSQL generations, with current semantics; serving materializes in the same database | Delta and delta-rs are deleted |
| 2 Facts | Extraction emits L0/L1 observations: occurrences, entities, places, call facts with origin, occurrence-keyed atoms | Extraction writes no legacy relation |
| 3 Normalized relations | Owner rule, call policies, resolutions, effective callables, the one binder, role-generated catalog | Legacy derivations are deleted |
| 4 Analysis and catalog | Ablation triage first. Then transfers, conditions, obligations, derivations, behaviors, summaries, models, field links, associations, selection, analytics and projections | Every producer writes only new relations |
| 5 Serving | Generated views, grants and wire contracts; native inputs from relations; full gates, pilot, MCP journeys and generation cutover | No adapter, legacy declaration or `cpg-schema` remains |

**Legacy adapters**
- Each adapter is a declared stage that computes one legacy relation from new relations.
- It lists its **declared quirks**: legacy behaviors deliberately reproduced, such as a legacy call-edge
  variant.
- It lists its **declared divergences**: intentional differences, each tied to a review finding.
- An undeclared difference is a failure.
- An adapter is built only when a legacy consumer outlives its producer by at least one phase.
  Otherwise the producer and consumer migrate together.
- An adapter is deleted in the phase where its last legacy consumer cuts over.

**Legacy identities.** Where a new identity recipe differs, the new producer emits a temporary
legacy-ID side relation. Adapters then reproduce legacy IDs exactly. The side relations are deleted in
phase 5.

**Parity.** Each phase dual-runs the legacy producer and the new producer plus adapters in one attempt.
Parity is established on:
- all fixtures;
- the pilot, in both profiles;
- served MCP output.

The comparison is exact equality of every legacy relation (rows and IDs), modulo declared divergences.
The legacy producer is deleted only after parity passes. Independent semantic controls migrate with
their layer; parity never replaces them.

**Research engines.** At phase-4 entry each Stage-3 research engine and each analytics variant is
disabled in turn:
- If served output and the retained regression controls are unchanged, it is retired, with operator
  confirmation. Git keeps it, and its ADR is superseded.
- Otherwise it migrates.

**Product and reviews**
- PR6 and new product features wait for phase 5. ADR-0071's product priority then resumes.
- There is no rollback path; a failing phase is fixed forward.
- Reviews:
  - a design/target review at the phase-0 and phase-5 exits;
  - a change/conformance review at the phase 1–4 exits.

## Consequences

- **What gets easier.**
  - Every legacy semantic, including the review's eight call-edge variants and four binders, becomes
    an explicit declared quirk or policy.
  - A divergence is attributed to exactly one layer.
  - Completion is objective at each phase.
  - The product keeps working on adapter output throughout.
- **What gets harder.**
  - Dual-running roughly doubles compile work during a phase.
  - Adapters and legacy-ID relations are throwaway code.
  - Product features pause until phase 5.
- **Where the findings go.** Review F01–F13 close by construction in the phases the
  [cutover plan](../plans/semantic-model-cutover-plan_2026-09-29.md) names; it owns their disposition,
  work packages and deletion obligations.
- **Status.** Implementation is Proposed; each phase exit establishes its part.
