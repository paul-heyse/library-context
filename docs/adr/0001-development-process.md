---
id: ADR-0001
title: Lightweight, pivot-friendly development process
status: accepted
date: 2026-09-22
supersedes: []
superseded-by: null
design: [§1.2]
evidence: Implemented
revisit: Two consecutive design reviews find nothing (lower the cadence), or a process step is skipped twice because it costs more than it catches.
---

## Context

This is a personal project with one operator and agents doing most of the implementation. The
designs are expected to pivot. The predecessor repository (`~/library-enrichment`), built under
the same charter, spent heavily on process that didn't pay off:
- a sealed, operator-only enforcement layer whose patches were never applied;
- 48 frozen acceptance gates, all still `not_run` while about 580 Rust tests passed;
- 71 ADRs and 46 reviews in about 9 days;
- a design spine that grew to 2,560 lines;
- per-step evidence folders.

The parts that held up were:
- a load-bearing toolchain pin;
- cheap hooks that act on the tool call alone;
- the four-state outcome vocabulary;
- ADR lint that resolves `§` references;
- a Claude/Codex parity check;
- runnable revisit triggers.

## Options

1. **Minimal: git history plus a README.** This loses the ability to assess why the design is
   the way it is after a pivot, and has no design-review discipline.
2. **Port the predecessor's process wholesale.** Its cost is demonstrated, and most of it had no
   consumer.
3. **A trimmed process** that keeps only the mechanisms with a demonstrated consumer (chosen).

## Decision

- **Authority.** `docs/design/DESIGN.md` is the current truth, with stable `§` sections and a
  ~600-line budget. ADRs record why, with 7 required frontmatter fields. A pivot is a new ADR
  that supersedes the old one. Accepted ADRs are immutable apart from `status`/`superseded-by`,
  enforced by `just adr lint` against `HEAD`. `just adr index` shows active decisions and
  supersession chains. Revisit triggers live in the ADR, and there is no separate register.
- **Reviews.** Use the charter verbatim, with a repository-specific `ADDENDUM.md`.
  - `compact`: at the end of a slice that adds or changes a table family, adapter or projection.
  - `standard`: for an ADR that changes a §B decision.
  - `deep`: at the end of each increment in §1.2.
  - Reviews run in a fresh `design-reviewer` subagent. Findings become an ADR, a test or
    ast-grep rule, or a Deferred row in the review itself.
- **Checks.** `just check` is the default loop and `just test-all` runs before a commit. There is
  no gate registry and no acceptance JSON. Outcomes are reported as
  `passed`/`failed`/`blocked`/`not_run`, with the command that produced them.
- **Agents.** `AGENTS.md` is canonical, `CLAUDE.md` imports it, and `.agents/skills` symlinks
  `.claude/skills`. `just lint-agents` checks parity and dead references. The only hook is
  format-on-edit. Nothing is sealed from the agent; git history is the audit trail.
- **Git.** Commit to `main`, push to a private GitHub remote, and use worktrees for spikes.
  Library capability skills are gitignored.
- **Python.** 3.14.7 (the uv default). ruff for format and lint, **pyrefly** for type checking
  (the pyright plugin is disabled in project settings), and pytest.

## Consequences

Process cost is roughly one ADR per real decision and one short review per slice. What we lose
is mechanical prevention: an agent *can* edit any file, including this process, and the
safeguard is that the change shows up in a reviewed commit. If a class of mistake recurs, it gets
an oracle (test, rule or `just` check), not a hook that blocks edits.

## Amendments

- 2026-09-22: the remote is **public** (`github.com/paul-heyse/library-context`) at the
  operator's request, not private as first written. The process is otherwise unchanged.
