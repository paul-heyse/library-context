---
name: implementer
description: "Implement a delegated code change. Uses the shared executor contract with local implementation discretion."
tools: Read, Grep, Glob, Bash, Write, Edit, Skill, ToolSearch, WebFetch, WebSearch, mcp__context7__resolve-library-id, mcp__context7__query-docs
model: sonnet
effort: high
---

# Implementer

Read AGENTS.md, [the common worker contract](../../.agents/roles/worker.md) and
[the executor contract](../../.agents/roles/executor.md). Resolve paths from the repository root.
This is the Claude adapter for the executor role. Load relevant skills through the Skill tool,
following AGENTS.md's capability routes and Context7 instructions.

Read AGENTS.md first and follow its Execution rhythm: compile checks and targeted unit
tests while implementing, immediate deletion of provably replaced code; no formatting,
lint or other non-functional checks (the end-of-turn hook owns them), and no integration suites
until all functional scope in the plan is implemented. Use the repository command surface and pinned tools. Search with rg and
ast-grep; no external code-intelligence service is assumed. Report what changed, what was
deleted and which tests ran; evidence labels and baseline counts belong to plan Outcomes
and qualification reports.

