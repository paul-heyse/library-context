---
id: ADR-0112
title: library-research writes its evidence and assigned shared skills; Codex role files carry no sandbox settings
status: accepted
date: 2026-10-01
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: A Codex release that applies sandbox settings from custom agent files again, or a library-research write outside its permitted scopes, warrants revisiting enforcement.
---

## Context

[DESIGN §1.2](../design/DESIGN.md#section-1-2) (ADR-0109) gives the shared role contracts the
responsibilities and the native definitions the provider settings. The library-research contract
said "Do not edit repository files", Claude's definition had no Write/Edit and Codex's declared
`sandbox_mode = "read-only"`. Research that should leave durable evidence (a probe and README in
`docs/design_review/evidence/`, or a fix to a shared library skill under
`~/.local/share/library-skills/`) was therefore redone by the coordinator or an executor.

Verified 2026-10-01 against Claude Code 2.1.287 documentation and the codex-cli 0.159.2 source:
Claude path permission rules and its Bash sandbox are session-wide (only the tool list and hooks
are per agent), and Codex drops `sandbox_mode`/`writable_roots` from custom agent files since
0.149, so every role inherits the session's sandbox. The operator asked for the same rule across
the repositories and their shared template (v0.3.1).

## Options

1. **Keep library-research read-only** — simplest, but evidence work keeps passing through a second
   agent, and the Codex declarations would still claim an enforcement that does not exist.
2. **Enforce write scopes natively** — Codex cannot (role files cannot carry sandbox settings);
   Claude only through a PreToolUse guard keyed on `agent_type`, which still cannot stop writes
   through the shell.
3. **One instruction, identical in every repository and the template, with the native tool grant
   (chosen).**

## Decision

The [library-research contract](../../.agents/roles/library-research.md) permits, within the
coordinator's brief, writes to the library-evidence locations AGENTS.md names (one new dated
folder per investigation with its README and Index row), to the source of a shared library skill
the brief assigns (repository-independent, one writer per skill), and to scratch outside the
repository; everything else stays read-only, and the role commits nothing unless assigned. A role
contract may grant standing write scopes that a brief can narrow
([worker contract](../../.agents/roles/worker.md)). Claude's definition lists Write and Edit; Codex
role files carry no `sandbox_mode`, and the [roles README](../../.agents/roles/README.md) states that
Codex roles inherit the session's sandbox. ADR-0109 is otherwise unchanged.

## Consequences

Evidence and skill improvements no longer need a second worker. The write scope is an instruction,
not an enforced boundary, in both runtimes: a misdirected write shows in the return (which lists
every file written) and in `git status`. The other evidence roles keep their read-only contracts.
Implemented in this commit; the template and the other repositories carry the same texts.
