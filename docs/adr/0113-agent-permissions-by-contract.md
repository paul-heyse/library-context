---
id: ADR-0113
title: Role contracts are the only bound on agent effects; native role files carry no tool lists or sandbox settings
status: accepted
date: 2026-10-01
supersedes: [ADR-0112]
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: A write outside a role's permitted scopes, or a runtime gaining per-agent write scoping that would enforce the contracts, warrants revisiting enforcement.
---

## Context

[DESIGN §1.2](../design/DESIGN.md#section-1-2) (ADR-0109) gives the shared role contracts the
responsibilities and the native definitions the provider settings. ADR-0112 let `library-research` write
its evidence and an assigned shared library skill by instruction, granted it Write/Edit through its
Claude tool list, and removed the Codex `sandbox_mode` keys, which Codex ignores in custom agent
files since 0.149. The other Claude adapters kept tool allowlists.

On 2026-10-01 a `library-research` subagent's evidence write of `report.md` was refused. That
refusal is built into Claude Code 2.1.287: the Write tool rejects, in any subagent, a `.md` file
whose name starts with report, summary, findings or analysis (case-insensitive). No setting or
permission rule disables it; other names, Edit and Bash are unaffected. The operator asked to remove
every native restriction on the agents and rely on the instructions, which were already the only
real boundary: Claude permission rules and its Bash sandbox are session-wide, and Codex roles
inherit the session's sandbox. The same change is made in the copier template and the other
repositories that share these contracts.

## Options

1. **Keep per-role tool allowlists** (ADR-0112). They restrict only Claude, cannot bound shell
   writes, and each new need (a role leaving a README) means another configuration change in every
   repository.
2. **Per-role guard hooks.** Partial (Write/Edit, not Bash; not Codex) and a hook per repository to
   maintain; rejected with ADR-0112.
3. **Contracts only, identical everywhere (chosen).** Native role files carry model and effort, and
   nothing that restricts tools or sandbox.

## Decision

The [shared role contracts](../../.agents/roles/README.md) alone bound each role's effects.
Claude adapters in `.claude/agents/` carry no `tools` list, so every role inherits the session's
tools; Codex role files carry no `sandbox_mode`, so every role inherits the session's sandbox and
approval settings. Within the coordinator's brief, the
[library-research contract](../../.agents/roles/library-research.md) permits writes to the
library-evidence locations AGENTS.md names (one new dated folder per investigation with its
README and Index row), to the source of a shared library skill the brief assigns (repository-independent, one
writer per skill) and to scratch outside the repository; it commits nothing unless assigned. The
other evidence roles' contracts stay read-only. A role contract may grant standing write scopes
that a brief can narrow, never widen ([worker contract](../../.agents/roles/worker.md)). The worker
contract also tells workers to name a markdown file for its content (`README.md`, `<topic>.md`),
because Claude Code's built-in subagent report-file refusal cannot be switched off. ADR-0109's
allocation of responsibilities and model settings is otherwise unchanged.

## Consequences

Evidence and skill improvements need no second worker, and no adapter change is needed when a role's
work changes. Every permission is an instruction in both runtimes: a misdirected write shows in the
return (which lists every file written) and in `git status`, and the session's own permission mode
and deny rules still apply to every subagent. Implemented in this commit for this repository; the
template and the other repositories carry the same texts. Revisit if a write goes astray or a
runtime gains per-agent write scoping.
