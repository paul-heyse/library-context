---
name: plan-execution
description: Develop a conversational approach to executing an existing approved plan, grounded in current scope, dependencies and acceptance evidence.
---

# Plan the execution approach

Develop an adaptable approach to carrying out the existing plan. Resolve the plan and the
user's authorized scope from the conversation and current repository context. Read `STATUS.md`,
the plan's relevant packages and receipts, and enough current code and architectural owners to
establish the actual baseline. Preserve concurrent work and distinguish implemented scope,
accepted decisions and proposals still awaiting resolution.

Build on the approach the user has already supplied. Useful lenses include:

- Which packages are authorized, their prerequisites, and assumptions that could change the work.
- Which contracts and ownership boundaries allow independent work, and where integration must wait.
- Which uncertainties need code mapping, library evidence or a bounded design decision.
- Where implementation review and focused functional checks will establish acceptance, and
  which review or integrated qualification belongs at the enclosing boundary.
- What migration, retirement or preservation obligations affect the sequence.

These are optional lenses, not a required pipeline or another plan template. Use the
[shared agent roles](../../../.agents/roles/README.md) when bounded delegation would help:
`code-mapper`, `library-research`, `design-reviewer`, `executor`,
`implementation-reviewer` and `test-agent`. Choose only the roles that serve the work; the root
coordinator retains design decisions, integration and acceptance. Delegate when independent work, context isolation, distinct capabilities or independent judgment
justify handoff and integration cost; small or tightly coupled tasks may stay with the root. Identify edit ownership and
dependencies before proposing parallel implementation.

Follow the repository binding's
[review cadence](../../../docs/design_review/design_principles/binding/library-context.md#reviews-in-this-repository)
for the relevant boundary. Routine implementation does not trigger recurring architecture or
domain-model assessment. The focused foundation assessment belongs to
[create-plan](../create-plan/SKILL.md); reopen a consequential question during execution when
new evidence warrants it. Library questions follow the repository's capability-skill and
documentation routes.

Keep the verification approach consistent with `AGENTS.md`: compile checks and targeted
functional tests during implementation; integrated tests and pilots after all functional scope
of the authorized plan is implemented. Formatting, linting and other hygiene are owned by the
automatic end-of-turn hook, not this workflow.

Present the approach in the conversation, explaining the material sequencing choices,
assumptions and unresolved prerequisites. Use the available planning interface when useful.
There is no separate durable execution plan by default; the existing plan remains the owner
of execution status and findings. Ask only for information that materially changes the work.
Follow the user's boundary between planning and action: this skill does not switch modes or
force execution. When execution is authorized, [execute-plan](../execute-plan/SKILL.md) supplies
the execution contract without requiring another planning round.
