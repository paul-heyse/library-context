---
id: ADR-0046
title: Pinned uv acquisition and an in-process Pyrefly/Ruff front end define the analysis inputs and run identity
status: accepted
date: 2026-09-25
supersedes: [ADR-0012, ADR-0013]
superseded-by: null
design: [§1.2, §1.4, §B1, §B8, §3.2, §3.4, §3.4.1, §3.5, §3.6, §4.0, §4.1, §4.2, §11, §12, §13]
evidence: Tested
revisit: The harness-equivalence test fails at a pin bump; a Pyrefly release needs a patch with a logic change (anything beyond visibility, borrow-only accessors and upstream-default fields) or over ~60 changed lines; a bump changes more than ~300 lines of the extractor's Pyrefly-facing module; Pyrefly panics on an in-scope library, or a pin cannot co-resolve with the §7 family (route: the sidecar alternative); the same inputs (the same verified analyzer-readable bytes, lock, interpreter and producer) give a different node_id or fact_id, or an analyzer answer changes without context_id changing; a library cannot be expressed as one hash-locked uv requirement with a first-party distribution list; `uv sync --frozen` stops verifying artifact hashes; or a release's modules are not all listed in its distributions' RECORDs.
---

## Context

This record replaces ADR-0012 and ADR-0013 with their surviving clauses, organized by
responsibility; the storage and identity-encoding clauses they also carried are restated in
ADR-0047. It governs how a library becomes analysis input and which front ends turn that input
into facts ([§4](../design/sections/acquisition-and-extraction.md#section-4), §B1, §B8).

Two forces shape the front end. Facts must come from the analyzer's own resolution, public-name
definition and parse, in one byte coordinate system, so that syntax, calls and exports join
without a second interpretation. And Pyrefly's call resolution is reachable only inside the
crate: the Pysa collectors reach the solver through `pub(crate)` entry points, and `report` is
private, so an unpatched dependency cannot supply calls. A spike on 2026-09-22 (FastMCP, 257
modules, plus a BOM/CRLF fixture) established the in-process route: Pyrefly co-resolves with the
pinned DataFusion/Arrow/delta-rs family; the configured `ConfigFile` spawns no interpreter and is
byte-identical under perturbed ambient variables; output is identical across reruns, shuffled
module order and thread counts; in-process call graphs equal the CLI's Pysa JSON for every module;
and every Pysa location converts back to its byte range exactly.

Two forces shape acquisition. The libraries compiled are generally not this project's
dependencies, and the project environment carries serving and model dependencies unrelated to the
analyzed code, so it must never be an input. And uv already pins every distribution of a closure
with its artifact hashes, verifies them on `uv sync --frozen`, and builds an isolated
environment, while every installed distribution's `RECORD` lists its files with their sha256.

## Options

Front end:
1. **Pyrefly as a subprocess, decoding its reports.** It analyzes twice (`check` plus coverage),
   needs a decoder and schema for undocumented JSON plus our own line tables, and the syntax walk
   parses a second time with no guarantee it matches Pyrefly's parse.
2. **An unpatched git dependency.** It reaches AST, bindings, exports, MRO and signatures, but not
   call resolution, so calls would still need the subprocess and every run would analyze twice.
3. **A minimally patched fork linked in-process** (chosen).
4. **The patched fork in a sidecar process emitting Arrow IPC.** The fallback if a future pin
   cannot co-resolve with the family or panics become common. It does not reduce the port cost, so
   a port-cost trigger routes to rebasing the fork instead.

Acquisition:
1. **Introspect the project's own `.venv`.** Rejected: analyzed libraries are not project
   dependencies, and that environment's contents are unrelated to the analyzed code.
2. **A bespoke acquisition tool** (download wheels and a tarball, write a manifest, build a venv).
   Rejected: it re-implements resolution, hash checking and installation, and adds a second lock
   format.
3. **One committed uv project per library; acquisition is `uv sync --frozen`; Stage A reads the
   acquired environment** (chosen).

## Decision

**Libraries are data.** Every analyzed library, the pilot included, is a directory
`libraries/<name>/` with three committed files:
- `pyproject.toml`: a virtual uv project with exactly one pinned requirement, an exact
  `requires-python`, and `[tool.lctx] release`, the first-party distributions whose code is
  compiled. Everything else installed is dependency context, analyzed only as far as imports
  reach. `[tool.lctx.source]` names the upstream repository, tag and full 40-hex commit for docs,
  examples and tests; the tag must name the locked version. Corpus selection semantics are
  ADR-0018's; example and test roles are ADR-0015's.
- `.python-version`: the exact interpreter.
- `uv.lock`: the reviewed acquisition lock.

**The pilot and the gold reference.** The pilot is FastMCP 4.0.5 in `libraries/fastmcp`, installed
with the `fastmcp` skill's install line (`fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`),
release `fastmcp`, `fastmcp-slim` and `fastmcp-tasks`. `scripts/check_gold.py` (`just gold`) fails
when the skill's install line or its resolved release versions differ from `libraries/fastmcp`, so
the gold reference and the analysis stay on one version. The skill is only ever an evaluation
reference, never a compiler input.

**Acquisition** is `lctx acquire <name>`: `uv sync --project libraries/<name> --frozen
--no-install-project --no-config --python <.python-version> --link-mode copy` into a gitignored,
rebuildable `build/envs/<name>`, with every other `UV_*` variable and `VIRTUAL_ENV` removed.
`--reinstall` is the remedy when Stage A finds a changed file. `lctx library init` locks without
`--no-config`, because the library's own `[tool.uv]` applies there and the lock is reviewed. When
a source is declared, `lctx acquire` also fetches the tree hermetically (no `GIT_*` variables,
system or global configuration, prompts or ambient attributes) as a shallow fetch of the one
commit, checked by `rev-parse HEAD` every time.

**Stage A: one equivalence, the analyzer-readable bytes.** The analyzer-readable files are `.py`,
`.pyi` and `py.typed`, the files Pyrefly's module finder reads; `.pth` files are never read.
Stage A reads the definition, the lock, `pyvenv.cfg` and every `*.dist-info`, with no network and
no interpreter, and refuses the attempt unless every installed distribution is the version the
lock names, the interpreter is `.python-version`'s, every distribution's analyzer-readable `RECORD`
entries match their sha256, and every release distribution has locked artifact hashes. The
release's modules are exactly the `.py`/`.pyi` entries of its distributions' `RECORD`s; nothing
walks a directory.

**Input and run identity** (encoding and content ids: ADR-0047).
- `release_id` hashes the release distributions' names and versions and the sorted (path, sha256)
  of their verified analyzer-readable entries: what is analyzed, not the lock entry. A source tree
  hashes its label. A corpus release hashes `repository@commit`, the library's `release_id`, and
  every selected file's path, content and role.
- `context_id` covers the Python version and platform, the ordered search and site-package paths
  relative to their roots, the digest of every configuration an analyzer receives, the
  **environment digest** (the installed distributions' dist-info names and every analyzer-readable
  file's site-relative path and content; `RECORD` lines outside site-packages never enter it) and
  the **lock digest**. Where a checkout or environment sits never changes an identity; what the
  analyzer can read must.
- `producer_id` covers the tool, its revision (for the extractor: the fork revision and patch
  digest, the ruff line, and the flow provider with its runtime-view version) and an adapter build
  digest bumped whenever mapping output changes.
- `run_id` covers release, context, producer, the sorted enabled families and the producer's own
  configuration digest.

**Runs in an attempt.** `releases`, `distributions` and `source_files` are written once per
extractor run, which carries Stage A's output; later producers reference `release_id` and never
append. An attempt holds several extractor runs only over distinct releases: the library, and the
corpus built from its upstream tree. The corpus run analyzes in the library's environment and
lists the same `distributions`, but has its own context, because its search path puts the tree
ahead of site-packages so that a module both runs import resolves to one file. What both runs
assert about one thing is one node, from its first fact.

**The front end.**
- **Pyrefly** is a git dependency on a fork that is an upstream tag plus one patch kept in
  `third_party/pyrefly-<ver>.patch`, pinned by revision. The patch may change visibility, add
  accessors that borrow or compose existing upstream queries, and add fields whose default
  reproduces upstream behaviour; nothing else. `just deps` checks that `Cargo.lock`, the driver's
  constants and `docs/pins.md` name one revision equal to tag plus patch. The revision, patch
  digest and their verification live in `docs/pins.md`, not here.
- **Ruff** crates are pinned to the line Pyrefly compiles against, so AST types are shared; the
  family check holds ruff, Pyrefly and blake3 to single versions.
- **The driver** builds an explicit `ConfigFile` (explicit search and site-package paths, Python
  version and platform; no discovery, heuristics, fallback search path or interpreter query), uses
  `ConfigFinder::new_constant`, runs `State::new(.., ThreadCount::Inline)` on a driver-owned thread
  with a declared stack, and runs `Require::Everything` with a no-write `PysaReporter` kept
  installed while dependency modules solve lazily. It refuses to start when
  `PYREFLY_STACK_SIZE`, `PYREFLY_FIXPOINT_DETAILS` or `PYSA_DUMP*` is set.
- **One parse of record.** The Ruff walk runs over `Transaction::get_ast`. The Pysa collectors
  supply calls, definitions and ancestry, converted to byte ranges through each module's own
  `LineIndex`. Every Pysa variant has a mapped row or an explicit "not carried" row, through
  exhaustive matches; `Overrides(f)` is a candidate with an open candidate set.
- **Public names** are Pyrefly's: (access path → origin) pairs from `trace_export_origin` must
  flatten exactly to `compute_public_fqns` in every run. A completeness detector marks `exports`
  `partial` when `__all__` is unresolvable or non-literal; Ruff does not corroborate public names.
- **One second parser line, confined to `cpg-flow`.** The flow facts come from ty's semantic index
  over a second parse at the ty/ruff line that crate pins (a declared extra family under ADR-0002),
  of the text Pyrefly read with every `TYPE_CHECKING` name token renamed to a same-length sentinel.
  Pyrefly's parse stays the parse of record: every syntax id, span and fact outside the `flow`
  family comes from it, and flow facts join ours by module and byte range under parity rules. Only
  byte ranges, place text and our condition data cross the crate boundary. Why ty supplies flow
  is ADR-0045's.
- **Panics abort the attempt.** Any panic in code that touches Pyrefly aborts it, with no
  `catch_unwind`: Pyrefly treats its state as unsupported after a panic.
- **The Pyrefly CLI of the same version** is only the harness-equivalence oracle, never a
  production input.

**One CLI, `lctx`,** is the production path: `library init`, `acquire`, `compile` (acquire,
Stage A, extract, derive, validate, publish) and `query`. Upgrading a library is: edit the pin,
`uv lock --upgrade-package`, review the lock diff, compile.

## Consequences

- **Easier.** One run, one parse of record and one coordinate system, with no JSON decoders;
  upstream enum changes fail our build instead of silently degrading a decoder; native Pyrefly
  types are reachable when a consumer needs them. Adding a library is data, not code.
- **Harder.** Each Pyrefly upgrade rebases the fork and ports against private APIs
  ([§4.2.6](../design/sections/acquisition-and-extraction.md#section-4-2-6)); a Pyrefly panic aborts
  the compile; the build carries jemalloc's C build and Pyrefly's bundled stubs; the first
  acquisition needs the network and every library has an environment on disk; the skill must be
  re-pinned with the library.
- **Evidence (Tested, 2026-09-22 onward).** `crates/cpg-extract/tests` (the variant table, keys,
  `__all__` forms, `_invalid/` modules, BOM/CRLF offsets, install-location and environment
  independence, module-order and cross-process determinism, the id recipes, the panic abort, the
  ambient refusal, harness equivalence with the CLI); `crates/cpg-extract/tests/library.rs` (drift,
  hash-less and changed-byte refusals, content-derived `release_id`, location-independent
  environment digest that sees unowned stubs, loose files and lock-only changes, the pinned
  source); `crates/lctx/tests/acquire.rs` (uv's flags and scrubbed environment);
  `crates/lctx/src/propose.rs` (release proposal); `cpg-flow`'s known-answer tests.
- **Known gaps** (the plan owns their disposition,
  [§6](../plans/behavioral-model-forward-plan_2026-09-24.md#6-findings-disposition)): the corpus
  run's search root is outside context and run identity, so an unselected helper a selected example
  imports can change facts without an identity change; this is an instance of this record's
  "answer changes without `context_id` changing" trigger, confined to the corpus root (W8). Pyrefly's
  resolved function flags are not persisted, and ty runs with empty program settings (W14).
