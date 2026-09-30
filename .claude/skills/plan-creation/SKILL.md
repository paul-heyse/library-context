---
name: plan-creation
description: Plan how to create an implementation-plan document from a design review or existing plan, including the investigations and design questions that matter.
---

# Plan the creation of a plan

Develop an approach to producing the plan the user needs. Load [create-plan](../create-plan/SKILL.md)
for the authoring contract and its suggested document structure, and follow its routes to the
applicable design-review skills, principles and repository binding.

Identify the reference review or existing plan and explore enough current repository context
to make the approach specific. Build on any approach the user has already supplied. Choose
the scope, investigation strategy and depth according to the intended outcome.

Useful considerations may include:

- Which obligations and design questions the source material leaves to resolve.
- Which existing components and contracts merit focused assessment in light of the proposed work.
- What current code, architectural owners, library capabilities or adjacent consumers can inform
  those decisions.
- Where source inspection is sufficient and where a scenario, probe or independent review may help.
- How much design detail and execution structure the eventual document needs.

These are suggestions, not required sections or an exhaustive investigation sequence. The
focused dependency assessment remains part of plan creation; choose how best to conduct it.

Use the [shared agent roles](../../../.agents/roles/README.md) for bounded investigation where useful.
Consider which code or library questions can proceed independently, where focused design advice
would help, and who will assemble the document. The coordinator owns the approach and design choices.

Produce an adaptable approach in the conversation, explaining the material scope choices,
assumptions and uncertainties. Ask questions only when their answers would meaningfully change
the work. Use the available planning interface when appropriate; otherwise present the approach
directly in the conversation.

Follow the user's requested boundary between planning the approach and writing the document
with `create-plan`. The conversational approach needs no separate durable document by default.
Implementation follows the authorization for the task.
