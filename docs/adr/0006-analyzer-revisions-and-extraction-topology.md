---
id: ADR-0006
title: Ruff crates in-process, Pyrefly 1.3.1 as a subprocess, one workspace
status: proposed
date: 2026-09-22
supersedes: [ADR-0003]
superseded-by: null
design: [§B1, §B8, §4.2]
evidence: Interface-checked
revisit: The increment-1 spikes fail (Pysa column units on non-ASCII source, sorted-output determinism, dependency resolution through the locked venv), or a pyrefly release after 1.3.1 is tagged and on PyPI with a needed report change.
---

## Context

The research input audited ruff git `660350be` and pyrefly git `9733bdcf` (1.4.0-dev.1). It
planned native adapters, including a hook into Ruff's crate-private `Checker` and Pyrefly's
`Require::Everything` state (IP L156–L166, L384–L409). ADR-0003 (proposed) put each analyzer in
its own workspace and process.

Evidence gathered 2026-09-22 (pyrefly-ruff skill; installed pyrefly 1.3.1; skill fixture
captures; pyrefly 1.2.0 source):
- **Pyrefly isn't published as a crate.** The name on crates.io is a squat.
  - At 1.3.1, its Glean, Pysa and CinderX converters are private.
  - Its state API is `#[doc(hidden)]` and marked unstable.
  - It depends on ruff crates `0.0.11` and jemalloc.
- **Pyrefly's CLI emits the reports:**
  - `check --report-pysa DIR --report-pysa-format json`: calls with 14 unresolved reasons,
    parameter kinds, MRO and types;
  - `check --report-glean DIR`: UTF-8 byte spans that line up with Ruff's `TextRange`;
  - `coverage report --public-only`.

  Report contents are stable across runs; the order of records is not.
- **Ruff library crates are published at `0.0.13`** (the ruff 0.16.7 line, MSRV 1.96), with no
  Arrow dependency. `SemanticModel`'s building blocks are public, but nothing public drives them.

## Options

1. **Link Pyrefly as a git dependency** (IP §5). Rejected: the report converters are unreachable,
   and it drags in a second ruff and jemalloc.
2. **ADR-0003: a separate workspace per analyzer.** Rejected. Once Pyrefly is a subprocess,
   isolation already exists, and Ruff's crates don't touch the Arrow family.
3. **The simpler alternative: Pyrefly reports only, no Ruff crates.** Rejected. No report covers
   guards, `if`/`raise` structure, argument-forwarding syntax, `__all__` contents or default-value
   syntax for Pass B/C.
4. **Ruff `=0.0.13` in-process; pinned Pyrefly 1.3.1 CLI; reports decoded in Rust; one
   workspace.** Chosen.

## Decision

- **Tools and topology.**
  - Ruff library crates are pinned `=0.0.13` and linked into a `cpg-extract` crate in the core
    workspace.
  - Pyrefly **1.3.1** (tag `3e3177d0f4755b56c2d5a710d830eed89b14c2e3`) runs as a pinned binary
    with a generated explicit config and never discovers configuration.
  - Pysa JSON and `coverage report` are decoded in increment 1. Glean is decoded only when a
    consumer needs cross-references.
- **Decoder rules** (DESIGN §4.2):
  - sort every record set;
  - normalize absolute paths;
  - convert Pysa `line:col` through line tables;
  - `artificial-call` sites become `synthetic_model`;
  - Pysa is authoritative for calls.
- **One producer per raw table** (DESIGN §3.2, §4.2).
  - Ruff and Pysa write separate raw tables. The merged `parameters`, `exports` and `call_sites`
    are DataFusion derivations.
  - "Public" is defined by Pyrefly `coverage report --public-only`. Ruff's `__all__`
    corroborates it, and a disagreement becomes a `provider_disagreement` boundary.
- **Ambient inputs are removed** (DESIGN §4.0).
  - Each release gets its own analysis venv, built from its own acquisition lock, never the
    project's.
  - The generated Pyrefly config points the interpreter into that venv, disables search-path
    heuristics and walk-up fallback, and fixes ignore-file handling.
  - The `pyrefly dump-config` digest is stored in `contexts`.
- **Binding history** comes from our own scope-aware recognizer over the Ruff AST. The
  conservative `ambiguous_binding` rule applies. Porting the Checker is deferred.
- **Binding-decision edits.** §B1 is revised (Ruff no longer "supplies lexical semantics") and
  §B8 is rewritten. `scripts/check_family.py` still guards the Arrow family.
- **Spikes before acceptance:**
  - Pysa column units on non-ASCII source;
  - Pysa size and decode time, on the fastmcp skill's existing cached captures and restricted to
    project files;
  - byte-identical Arrow output across two runs after sorting;
  - Pyrefly resolving FastMCP's dependencies through the separate 4.0.3 analysis venv;
  - `pyrefly dump-config` output identical with `PATH` and `VIRTUAL_ENV` perturbed.

## Consequences

- **Simpler build.** One workspace, one lockfile and no adapter IPC protocol.
- **What we depend on.** Report formats, not internals. A Pyrefly upgrade is a pin change plus
  decoder fixture updates, not an API port.
- **What we give up.** Native type structure and inference-explanation facts (IP L334–L409) until
  a consumer justifies a deeper integration (DESIGN §13).
