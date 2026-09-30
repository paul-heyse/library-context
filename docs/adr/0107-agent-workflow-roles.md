---
id: ADR-0107
title: Coordinate work through reusable roles and paired workflow skills
status: accepted
date: 2026-09-30
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: Repeated delegation overhead, missed contracts, model availability changes or measured resource use justify changing the role boundaries or runtime allocations.
---

## Context

Review and plan-authoring skills already separate preparing an approach from performing the work.
The operator approved extending that pairing to plan execution and using parallel subagents to
allocate investigation, implementation and verification more deliberately. Existing Claude agent
instructions duplicated process rules and included foreign repository routes. Workflow behavior
needs one maintained owner across Codex and Claude, while runtime model choices differ.

[DESIGN §1.2](../design/DESIGN.md#section-1-2) owns this development workflow. The review binding
continues to own cadence and acceptance; AGENTS.md owns functional-test timing and hook boundaries.

## Options

1. **Coordinator-only execution with ad hoc delegation.** Minimal configuration, but repeated
   context gathering and implicit model inheritance make resource allocation and parallel work
   harder to control. Retained as a reasonable choice for small or tightly coupled tasks.
2. **A fixed pipeline or many domain-specific executors.** Predictable assignments, but needless
   handoffs and duplicated instructions constrain useful decomposition and increase maintenance.
3. **Reusable roles, shared contracts and workflow skills (chosen).** The coordinator chooses
   useful boundaries; provider adapters supply native settings while shared contracts own behavior.

## Decision

A coordinator retains design, integration and acceptance. Six reusable worker roles provide code
mapping, library research, independent design review, execution, implementation review and functional
testing. Workers receive bounded assignments, exercise local judgment and return source evidence,
changes or check receipts. Material contract contradictions return to the coordinator. No fixed
decomposition or mandatory use of every role is introduced.

[Shared contracts](../../.agents/roles/README.md) own responsibilities and handoffs. Native definitions
in `.codex/agents/` and `.claude/agents/` own provider settings; Claude's existing `implementer` name
adapts the executor contract. Codex starts with Astra/high for coordination, Luna/high for evidence,
Sol/high for design and execution, and Sol/medium for implementation review and testing. Agents
use concurrency as they see fit and strive for parallel execution. Explicit user/runtime model
choices remain authoritative.

Three [skill pairs](../../.claude/skills/README.md) prepare and perform design review, plan creation
and plan execution. Preparation ordinarily stays conversational. Existing plans own execution and
scheduled findings. Focused assessment of foundations is integral to plan creation; required changes
enter the plan as prerequisites. Focused design advice is incorporated in the plan and cannot replace
a formal review due under the binding. A read-only formal reviewer returns the complete report for
the coordinator to publish without changing its judgment.

Parallel editing has explicit ownership and integration dependencies. Genuine concurrent production
editing uses separate worktrees. Reviews and checks identify a stable baseline. Functional-test
timing, preservation, migration rules and automatic non-functional checks retain their current owners.
No planning-mode hook, additional gate, findings ledger or standing implementation-time domain review
is introduced.

## Consequences

Provider configuration can evolve without rewriting each workflow. One general executor supports
different useful work partitions. Independent evidence and review can overlap implementation when
inputs are stable; shared build and database resources still need coordination.

Role contracts, adapters and skills are Implemented. Configuration discovery is recorded in
STATUS.md with its actual scope. Broad cost savings and workflow quality
remain Proposed: parallelism can increase total tokens, and allocations need calibration from use.
Native read-only defaults do not override a parent's live permission settings; the role's task
contract continues to bound allowed effects.
