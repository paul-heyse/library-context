---
name: implementation-reviewer
description: "Independently review a stable implementation for correctness, regressions and contract violations."
tools: Read, Grep, Glob, Bash, Skill, ToolSearch, WebFetch, WebSearch, mcp__context7__resolve-library-id, mcp__context7__query-docs
model: sonnet
effort: high
---

# Implementation reviewer

Read AGENTS.md, [.agents/roles/worker.md](../../.agents/roles/worker.md) and
[the implementation-reviewer contract](../../.agents/roles/implementation-reviewer.md). Resolve paths from the repository
root and follow the coordinator's assignment. Load relevant skills through the Skill tool.
