---
id: ADR-0171
title: Functional Codex roles inherit the operator-selected model
status: accepted
date: 2026-10-14
supersedes: [ADR-0109]
superseded-by: null
design: [§1.2]
evidence: Tested
revisit: A selected Codex model cannot support the required role or effort, or effective fresh-session inheritance differs from the operator selection.
---

## Context

The operator uses Codex and one selected model for this personal project. Project coordinator,
subagent-default and custom-role model pins competed with that choice. The
[friction review O04/RC05](../design_review/reviews/design_review_implementation-friction-target_2026-10-14.md#O04)
identified configuration divergence, not an observed model mismatch or comparative quality result.
The operator accepted single-model inheritance on 2026-10-14. This replaces ADR-0109's Codex model
allocation while restating its independent coordination contracts. It changes no Claude allocation.

## Options

1. **Inherited operator model with reusable roles (selected).** Removing competing model pins lets
   the runtime carry the selected model into every role; responsibility, effort and independent
   judgment remain explicit without duplicated model-selection state.
2. **Explicit model overrides at every spawn.** Workable where a runtime cannot inherit, but repeats
   policy and can be overridden by custom role files. Keep it only as a disclosed runtime fallback.
3. **Different models by role or a fixed workflow.** Rejected for this operator's chosen workflow;
   neither role separation nor parallel work requires a different model or mandatory agent count.

## Decision

A coordinator owns design, integration and acceptance. Six reusable worker roles cover code mapping,
library research, independent design review, execution, implementation review and functional testing.
Delegate when independent coverage, context isolation, capabilities or judgment justify coordination;
small/tightly coupled work may stay with the root. No fixed pipeline or requirement to use all roles.

The operator selects the Codex model in user/session configuration. Project coordinator/default-
subagent and custom-role files do not pin another model. Retain supported role effort settings and
explicit operator overrides. Configuration declarations and existing running threads are not proof
of effective fresh-session behavior. Qualify a fresh parent/role invocation; disclose any runtime
that requires an explicit selected-model override. This does not modify host/user configuration.

Shared role contracts own responsibilities and permitted effects; native definitions supply provider
instructions/effort. Claude keeps Sonnet/medium code mapping, Opus/medium library research,
implementation/review/testing, Opus/high design review and its existing implementer adapter. Parent permissions and live
authorization govern effects; native role declarations cannot override the parent's sandbox.
Library-research retains ADR-0113's assigned evidence/shared-skill write scope; other evidence roles
remain read-only. No model routing is added to shared library capability skills.

Briefs identify the exact/dirty baseline, useful outcome, settled decisions, sibling ownership,
permitted effects, dependencies and meaningful evidence. Workers start with instructions already in
context, relevant owners and the shared/role contracts, not the root's entire tour. Follow discovered
read-only dependencies. Negative claims state search scope and limits. Consequential absence,
conflicting evidence, unsupported version transfer, unresolved contracts or a repeated failed repair
return to the coordinator for reassessment, not automatic model escalation.

Paired process skills prepare/perform reviews, plan creation and execution. Preparation stays
conversational by default; existing plans own execution/disposition. Focused foundation assessment
belongs to plan creation; formal review still follows the binding. A read-only formal reviewer
returns its unaltered judgment for root publication. Identify stable review/test inputs and integrate
concurrent edits through explicit ownership, using worktrees for conflicting mutable work. No
planning-mode hook, second ledger, standing domain review or new acceptance gate is introduced.

Use task activation for new work on an idle agent; a message alone is not activation. Observe the
returned state, and do not infer message delivery delays from encrypted payloads. This is runtime
usage guidance, not a repository scheduler.

## Consequences

Role boundaries evolve independently of model choice. One model remains compatible with independent
review and parallel implementation; broader savings/effectiveness remain unmeasured. Fresh-session
qualification and current implementation/closure belong to
[Catalog coordinator §6](../plans/catalog-completion-plan_2026-10-14.md#6-sole-disposition-and-current-checkpoint).
Fresh Codex0.162.1 parent/role invocations on 2026-10-14 confirmed actual inherited `gpt-6.1-sol`
in both threads, and a separate explicit selected-model invocation passed. These controls qualify
configuration behavior, not calibration, resource caps or product acceptance. The
[official inheritance/override order](https://learn.chatgpt.com/docs/agent-configuration/subagents#custom-agents)
was inspected on 2026-10-14; actual thread metadata is a separate control.
