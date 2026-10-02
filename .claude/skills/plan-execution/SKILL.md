---
name: plan-execution
description: Develop a conversational approach to executing an existing approved plan, grounded in current scope, dependencies and acceptance evidence.
---

# Plan the execution approach

Develop an execution approach that makes the existing plan actionable in the current tree.
Concentrate advance reasoning on decisions whose consequences cross work boundaries,
prerequisites that determine readiness, and uncertainties that could substantially change the
work. Leave local choices with the implementer who will have the best evidence. When delegation
helps, shape assignments and context so workers can exercise independent judgment while
contributing to one coherent result.

Resolve the plan and authorized scope from the conversation and repository context, building
on the approach the user has supplied. Read `STATUS.md` for the current
checkpoint when relevant to the task.
Read the relevant packages and receipts, and enough current code and architectural owners to
establish what exists. Preserve concurrent work and distinguish implemented scope, accepted
decisions and proposals. The following principles guide attention; they are not required stages
or another plan template.

## Allocate advance thought by consequence

Consider a decision's reach, reversibility and cost of learning later. Resolve questions early
when they constrain several assignments, determine whether work can begin, or could cause
substantial rework. Shared failure semantics may need agreement before delegation; an internal
representation can often wait for implementation evidence.

Choose the least investigation that can change the approach. Source inspection, library
evidence or a bounded decision may settle a prerequisite. Some uncertainty is more cheaply
resolved by implementing a bounded part of the actual target. Preserve the enclosing scope and
its remaining obligations when using such a slice to learn.

## Shape assignments around responsibility and readiness

Document boundaries provide navigation; assignments group work that someone can usefully
complete with the inputs available. An assignment may span documents or cover one of several
consumers in a single document. Consider whether its owner can deliver a coherent result
without repeatedly negotiating the same unresolved decisions with siblings. If coordination
dominates, reconsider the split or establish the shared prerequisite first.

Delegation can supply independent judgment, isolate a demanding investigation or parallelize
ready work. Weigh these benefits against briefing and integration costs; tightly coupled work
may be simpler for the coordinator to handle directly. An agreed interface can enable consumer
preparation before the working prerequisite exists, but does not establish integration readiness.

Use the [shared agent roles and coordination contract](../../../.agents/roles/README.md) for
assignment mechanics, permissions, baseline and edit ownership. Choose roles that serve the work;
no agent count or role sequence is prescribed. The coordinator retains design decisions,
integration and acceptance.

## Give workers context for judgment

Explain why an assignment exists, the result others need from it, and the decisions left to its
owner. Select references for what they contribute: the relevant plan package supplies intended
behavior and completion boundaries; the architectural owner supplies applicable contracts;
a review finding can explain a diagnosis, failure scenario or consequential rationale; current
code and checkpoints establish the actual baseline.

Briefly explain why a reference matters rather than transferring the whole reading burden.
Preserve enough reasoning for the worker to recognize a locally attractive choice that would
undermine the combined target. A historical review's suggested remedy is not automatically
an accepted decision. Workers may follow discovered dependencies beyond the initial references.

For example, the reasoning portion of a persistence assignment might say:

> Implement the persistence consumer in package C against the resolved-identity contract in
> section X. Finding F-12 explains why reapplying current defaults loses meaning; use that
> rationale when choosing round-trip checks. Another worker owns the producer. Choose the
> internal storage mapping locally and raise any need to change the shared identity semantics.

This illustrates useful context and discretion, not a complete brief or mandatory format.
Include task-specific scope, permissions and baseline through the existing coordination contract.

## Anticipate divergence across boundaries

Disjoint files do not guarantee independent decisions. Workers can differ over identity,
ordering, absence, failure behavior or lifecycle assumptions while each satisfies a local reading
of the plan. Identify the shared assumptions that materially constrain parallel work and how
changes to them will reach affected workers. Keep local details with executors and cross-boundary
decisions with the coordinator.

The necessary agreement depends on what dependent work relies on; not every interface needs to
be frozen upfront. When a premise changes, reassess the affected assignments and evidence.
Coordinate shared editing surfaces, generators, build resources or test environments where
actual contention is likely, following repository policy.

## Plan how contributions become a working whole

Consider who will connect contributions, migrate adjacent consumers, retire replaced paths and
establish the combined behavior. Account for the plan's preservation and transition obligations
when assigning that work. Ask what could remain wrong even if every worker satisfies its local
brief; the answer identifies where coordinator attention or a revealing cross-boundary check
would help.

Choose useful points for implementation review and functional feedback against identifiable
inputs. A worker's completion report should expose remaining assumptions, evidence limits and
integration obligations. Inspect decisive evidence and reconcile contributions before accepting
the combined result. Local completion and enclosing qualification establish different claims;
handoffs do not create additional qualification gates.

Follow `AGENTS.md` for verification scope and timing, generation, formatting and hook ownership.
Use focused checks during implementation and reserve integrated qualification for the boundary
specified by repository policy.

## Keep detail proportional to what is known

Prepare the first ready assignments concretely. Later work may need only an intended outcome,
a dependency and the condition that makes it actionable. Explain what would change the approach:
an unavailable prerequisite, a broader migration, a contract contradiction, or a split that
produces excessive coordination. Reconsider affected work while continuing independent
authorized scope.

Follow the repository binding's
[review cadence](../../../docs/design_review/design_principles/binding/library-context.md#reviews-in-this-repository).
The foundation assessment belongs to [create-plan](../create-plan/SKILL.md); ordinary execution
does not repeat it or initiate recurring architecture or domain-model reviews. Reopen a
consequential question when new evidence warrants it. Library questions follow the repository's
capability-skill and documentation routes.

Present material sequencing choices, assumptions and unresolved prerequisites in the conversation.
Use the available planning interface when useful. The approach is sufficiently detailed when
ready work can begin with understood boundaries and useful feedback, and further planning
depends on evidence implementation will produce. The existing plan remains the owner of
durable decisions, status and findings within the repository's authority structure; there is
no separate durable execution plan by default.

Ask only for information that materially changes the work. Follow the user's boundary between
planning and action: this skill does not switch modes or force execution. When execution is
authorized, [execute-plan](../execute-plan/SKILL.md) supplies the execution contract without
requiring another planning round.
