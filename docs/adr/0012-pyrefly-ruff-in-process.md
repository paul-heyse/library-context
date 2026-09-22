---
id: ADR-0012
title: Pyrefly 1.3.1 (patched fork) and Ruff 0.0.11 linked in-process as the fact front end
status: proposed
date: 2026-09-22
supersedes: [ADR-0006]
superseded-by: null
design: [§B1, §B2, §B8, §3.2, §3.4, §3.5, §4.0, §4.1, §4.2, §4.3, §6.1, §6.2, §8, §13]
evidence: Tested
revisit: The in-process vs CLI parity test fails at a pin bump; a pyrefly release needs a patch with a logic change or over ~60 changed lines; or pyrefly panics on an in-scope library.
---

## Context

The operator asked to link Pyrefly's and Ruff's internals directly, so code facts flow from
their in-memory structures into Arrow, DataFusion and Delta inside one Rust pipeline.

ADR-0006 (never accepted) rejected linking Pyrefly for four reasons:
- Pyrefly is not on crates.io.
- `mod report;` is private.
- `state` is `#[doc(hidden)]`.
- It pulls ruff 0.0.11 and jemalloc.

Source reading at 1.3.1 (pyrefly-ruff skill; pinned source) showed why calls need a patch.
The Pysa collectors reach the solver through `Transaction::ad_hoc_solve` and
`AnswersSolver::new`, and both are `pub(crate)`. Call resolution therefore cannot be rebuilt
outside the crate, but the collectors themselves become callable once `report` is `pub`.

Spike `spike/pyrefly-inproc` (`d00bab5`, `analysis/SPIKE_RESULTS.md`), 2026-09-22, stable
1.98.1, FastMCP 4.0.3 (257 modules) plus a non-ASCII BOM/CRLF fixture:
- **Build (S1).** Pyrefly builds in one graph with the pinned DataFusion/Arrow/delta-rs family.
  Every family crate resolves to one version, including ruff 0.0.11 and blake3 1.8.6. It needs
  the `lsp-types` git source and four permissive licenses (0BSD, ISC, Unicode-DFS-2016, BSL-1.0).
- **Ambient inputs (S2).** The configured `ConfigFile` spawns no interpreter (checked with
  `strace`). Output is byte-identical under a perturbed `PATH`, `VIRTUAL_ENV`, `PYTHONPATH`,
  `CONDA_PREFIX` and working directory.
- **Determinism (S3).** Five tables are byte-identical across reruns, shuffled handle order and
  4 threads.
- **Parity (S4).** In-process call graphs equal the CLI's Pysa JSON for 257/257 modules.
  Definitions are equal as sets; the only differences are the order of two set-valued lists.
  Public-name pairs flatten exactly to `compute_public_fqns`.
- **Coordinates (S5).** All 29,279 Pysa locations convert back to byte ranges exactly. The join
  key is the full call range, which matches 13,104 of 13,292 calls. The 188 unmatched calls are
  all inside annotations.
- **Delta (S6).** CHECK constraints and `appendOnly` are enforced on `DeltaTable::write`, while
  DataFusion `INSERT INTO` bypasses them. `delta.constraints.*` cannot be set at create.
- **Cost (S7).** 5.4 s and ~918 MB peak RSS at one thread.

## Options

1. **The simpler alternative: keep ADR-0006.** Pyrefly stays a subprocess and its JSON is
   decoded. This loses:
   - it analyzes twice (`check` plus `coverage`);
   - it needs a decoder and schema for undocumented JSON, plus our own line tables;
   - the syntax walk parses a second time, with no guarantee it matches Pyrefly's parse.
2. **An unpatched git dependency.** It reaches AST, bindings, exports, MRO and signatures, but
   not call resolution. Calls would still need the subprocess, so every run would analyze twice.
3. **The patched fork, in-process** (chosen). The patch is 33 changed lines of visibility, a
   `write_files` switch whose default keeps upstream behavior, and a `pysa_reporter()` borrow.
   It changes no logic.
4. **The patched fork in a sidecar process emitting Arrow IPC** (CodeFabric's shape). This is the
   **fallback** if a future pin cannot co-resolve with the family or panics become common.

## Decision

- **Source.** `github.com/paul-heyse/pyrefly` at `b9f28575`: tag 1.3.1 (`3e3177d0`) plus the
  patch kept in `third_party/pyrefly-1.3.1.patch`. It is a git dependency pinned by revision.
- **Ruff.** Every ruff crate is `=0.0.11`, the line Pyrefly compiles against, so AST types are
  shared. `check_family.py` checks ruff, pyrefly and blake3 for single versions.
- **Driver** (DESIGN §4.2.1):
  - an explicit `ConfigFile` with no discovery, heuristics, fallback or interpreter query;
  - `ConfigFinder::new_constant`;
  - `State::new(.., NumThreads(1))`;
  - `run(.., Require::Everything)` with a no-write `PysaReporter` installed.

  It refuses to start when `PYREFLY_STACK_SIZE`, `PYREFLY_FIXPOINT_DETAILS` or `PYSA_DUMP*` are
  set.
- **One parse** (§4.2.2). The Ruff walk runs over `Transaction::get_ast`. The Pysa collectors
  supply calls, definitions and ancestry, converted to byte ranges through the module's own
  `LineIndex`.
- **Public names** (§4.2.3). Pyrefly defines "public". Our (access path → origin) pairs come
  from its `trace_export_origin` and must flatten exactly to `compute_public_fqns`, which is
  asserted in every run. Ruff's `__all__` corroboration and the public `provider_disagreement`
  rule are dropped, because both sides would now be Pyrefly.
- **Ids.** Ids use the `blake3` crate, shared with Pyrefly at `=1.8.6`. §B2 allows `cpg-schema`
  to depend on `arrow-*` and `blake3`.
- **Storage built-ins** (§4.3, §6):
  - CHECK constraints are added with `add_constraint()`, and writes go through
    `DeltaTable::write` only;
  - DataFusion `INSERT INTO`/`write_table` into Delta is never used;
  - ids are read back with a two-step cast.
- **The CLI of the same version** is kept only as a parity-test oracle.

## Consequences

- **Easier.**
  - One run, one parse and one coordinate system; no JSON decoders.
  - Native Pyrefly types become reachable when a consumer needs them (§13).
  - Upstream enum changes fail our build through exhaustive matches, rather than silently
    degrading a decoder.
- **Harder.**
  - Each Pyrefly upgrade means rebasing the fork commit and porting against private APIs.
  - A panic in Pyrefly's run aborts the compile attempt.
  - The build is heavier: jemalloc's C build and about 41 MB of bundled stubs.
- **Carried into increment 1, slice 1:**
  - workspace dependencies;
  - the `deny.toml` git source and licenses (delta-rs as a normal dependency adds its own,
    independent of this ADR);
  - the family-check extension;
  - the parity test as a nextest test;
  - the `outside_provider_model` boundary reason for the 188 annotation-context calls.
