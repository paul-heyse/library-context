---
name: code-mapper
description: "Map repository owners, contracts, dependencies and consumers with precise source evidence."
tools: Read, Grep, Glob, Bash, Skill, ToolSearch, WebFetch, WebSearch, mcp__context7__resolve-library-id, mcp__context7__query-docs
model: haiku
---

# Code mapper

Read AGENTS.md, [.agents/roles/worker.md](../../.agents/roles/worker.md) and
[the code-mapper contract](../../.agents/roles/code-mapper.md). Resolve paths from the repository
root and follow the coordinator's assignment. Load relevant skills through the Skill tool.
