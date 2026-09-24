---
id: ADR-0012
title: Pyrefly 1.3.1 (patched fork) and Ruff 0.0.11 linked in-process as the fact front end
status: accepted
date: 2026-09-22
supersedes: [ADR-0006]
superseded-by: null
design: [§B1, §B2, §B8, §3.2, §3.3, §3.4, §3.5, §3.6, §4.0, §4.1, §4.2, §4.3, §6.1, §6.2, §6.3, §8, §13]
evidence: Tested
revisit: The harness-equivalence test fails at a pin bump; a pyrefly release needs a patch with a logic change (anything beyond visibility, borrow-only accessors and upstream-default fields) or over ~60 changed lines; a bump changes more than ~300 lines of the extractor's Pyrefly-facing module (route: Option 1); or pyrefly panics on an in-scope library, or a pin cannot co-resolve with the §7 family (route: Option 4).
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
- **Cost (S7).** 5.4 s and ~918 MB peak RSS at `NumThreads(1)`; 4.1 s and ~765 MB with
  `Inline`, which gives identical output.

The standard review (`design_review_adr-0012-pyrefly-ruff-in-process_2026-09-22.md`) found the
core sound and returned **Not Accept as written**. The operator accepted its four recommended
author decisions on 2026-09-22. The Decision below includes its corrections:
- abort on any panic (F1);
- a full Pysa variant table (F2);
- an `__all__` completeness detector (F3);
- immutable CHECKs verified at open (F4);
- the smaller corrections F5–F11.

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
   It makes no logic change in the §4.2.6 sense.
4. **The patched fork in a sidecar process emitting Arrow IPC** (CodeFabric's shape). This is the
   **fallback** if a future pin cannot co-resolve with the family or panics become common. It
   does not reduce the port cost, so a port-cost trigger routes to Option 1 instead.

## Decision

- **Source.** `github.com/paul-heyse/pyrefly`, branch `lctx/1.3.1`, at `b9f28575`. That is tag
  1.3.1 (`3e3177d0`) plus the patch kept in `third_party/pyrefly-1.3.1.patch`. It is a git
  dependency pinned by revision. The fork was published 2026-09-22. A fresh clone reproduces the
  patch's sha256, and the spike built from the git revision with output byte-identical to the
  local-clone runs.
- **Ruff.** Every ruff crate is `=0.0.11`, the line Pyrefly compiles against, so AST types are
  shared. `check_family.py` checks ruff, pyrefly and blake3 for single versions.
- **Driver** (DESIGN §4.2.1):
  - an explicit `ConfigFile` with no discovery, heuristics, fallback or interpreter query;
  - `ConfigFinder::new_constant`;
  - `State::new(.., ThreadCount::Inline)` on a driver-owned thread with a declared stack;
  - `run(.., Require::Everything)` with a no-write `PysaReporter` installed.

  It refuses to start when `PYREFLY_STACK_SIZE`, `PYREFLY_FIXPOINT_DETAILS` or `PYSA_DUMP*` are
  set. Context paths are recorded relative to declared roots.
- **Panics** (§4.2.5). Any panic in code that touches Pyrefly aborts the attempt, and there is no
  `catch_unwind`.
- **One parse** (§4.2.2). The Ruff walk runs over `Transaction::get_ast`. The Pysa collectors
  supply calls, definitions and ancestry, converted to byte ranges through the module's own
  `LineIndex`.
- **Pysa variants** (§4.2.3). Every variant has a mapped row, or an explicit "not carried" row.
  `Overrides(f)` is a candidate with an open candidate set (§3.6).
- **Public names** (§4.2.3). Pyrefly defines "public". Our (access path → origin) pairs come
  from its `trace_export_origin` and must flatten exactly to `compute_public_fqns`, which is
  asserted in every run. Ruff's `__all__` corroboration and the public `provider_disagreement`
  rule are dropped. In their place, a completeness detector marks `exports` as `partial` when
  `__all__` is unresolvable or non-literal.
- **Ids.** Ids use the `blake3` crate, shared with Pyrefly at `=1.8.6`. §B2 allows `cpg-schema`
  to depend on `arrow-*` and `blake3`.
- **Storage built-ins** (§4.3, §6):
  - only immutable CHECKs (span order, non-negative offsets) are added, with `add_constraint()`,
    and they are verified when a table is opened. Codebook membership stays a §8 validator;
  - writes go through `DeltaTable::write` only;
  - DataFusion `INSERT INTO`/`write_table` and the low-level writers are never used on fact
    tables;
  - ids are read back with a two-step cast.
- **The CLI of the same version** is kept only as the harness-equivalence oracle (§4.2.5).

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
- **Carried into increment 1, slice 1** (the review's oracles):
  - workspace dependencies;
  - the `deny.toml` git source and licenses (delta-rs as a normal dependency adds its own,
    independent of this ADR);
  - the family-check extension;
  - the harness-equivalence test;
  - fixtures and tests for: the variant table (insta), `__all__` forms, `_invalid/` modules,
    import-cycle determinism, two install locations, the CHECK verify, and a panicking fault
    hook;
  - ast-grep rules for `catch_unwind` and for the write and SQL bypasses;
  - a `just` recipe that checks the fork against tag plus patch, and the refused env list.

## Amendments

- 2026-09-23: H1 D6 (the library-leverage review). Fork revision `a07b7bae` (branch
  `lctx/1.3.1-r3`) adds `Answers::get_annotation(bindings, key)`, a borrow-only accessor composing
  `key_to_idx_hashed_opt` and `get_idx`, so the `types` family reads a declared return
  annotation directly instead of undoing Pyrefly's `async def` `Coroutine` wrapping; it removes
  the `Defs` re-walk and the inversion from `types.rs`. The patch is 51 changed lines, within this
  ADR's trigger. Earlier branches stay published.
- 2026-09-23: correction to the line above (the H1 review, F7). The patch's diffstat is 42
  insertions and 12 deletions, 54 changed lines, still within the trigger.
- 2026-09-24: **a second parser line, confined to one crate** (ADR-0022 §The flow provider; the
  behavioral-model plan's D-2; ADR-0002's declared extra families).
  - `cpg-flow` links `ty_python_core`, `ty_module_resolver`, `ty_vendored` and `ruff_db` at
    0.0.14, with ruff's own crates at 0.0.14 and salsa exactly at 0.28.2. It parses each release
    module a second time, from the text Pyrefly read with every `TYPE_CHECKING` name token renamed
    to a same-length sentinel, so no byte range moves (ADR-0022 §The flow provider).
  - **Pyrefly's parse stays the parse of record.** Every syntax id, span and fact outside the
    `flow` family comes from it. The ty parse contributes only flow facts, which are joined to ours
    by module and byte range under a parity rule.
  - **What crosses the crate boundary:** byte ranges, place text and our condition data. No ruff
    0.0.14 or ty type appears in `cpg-flow`'s API.
  - **Identity.** The provider and its runtime-view version are part of the extractor's
    `producers.revision`, so they are part of `producer_id` (ADR-0022, F9).
  - **Measured by the Stage 2.1 spike** on FastMCP 4.0.5's release, 2026-09-24: 213 ms and 47 MiB
    peak for 275 modules.
