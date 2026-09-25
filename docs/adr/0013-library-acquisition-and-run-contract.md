---
id: ADR-0013
title: Libraries are pinned uv projects; Stage A reads the acquired environment; the pilot is FastMCP 4.0.5
status: superseded
date: 2026-09-22
supersedes: [ADR-0007]
superseded-by: [ADR-0046, ADR-0047]
design: [§1.2, §1.4, §3.2, §3.4.1, §4.0, §4.1, §11, §12]
evidence: Tested
revisit: The same inputs (the same verified analyzer-readable bytes, lock, interpreter and producer) give a different node_id or fact_id, or an analyzer answer changes without context_id changing (carried from ADR-0007); a library cannot be expressed as one hash-locked uv requirement with a first-party distribution list; `uv sync --frozen` stops verifying artifact hashes; or a release's modules are not all listed in its distributions' RECORDs.
---

## Context

ADR-0007 fixed the run contract and the BLAKE3 identities. Its acquisition clause was never
built: it planned "a locked manifest plus `ACQUISITION.json`, the fastmcp skill's pattern" and a
separate analysis venv per release. The extractor still took a hand-made `release_root` and
hashed a `--release-label` into `release_id`.

On 2026-09-22 the operator decided three things:
- Pivot the pilot, the gold reference and the docs to **FastMCP 4.0.5**, with no version
  discontinuity.
- Acquisition is the **production** mechanism, the one way any Python library is pinned, fetched
  and compiled. Those libraries are generally not this project's dependencies, so the project
  environment is never an input.
- A new upstream version must be routine.

Two facts shaped the answer:
- **uv already does the hard parts.** A per-project `uv.lock` pins every distribution of a
  closure with its artifacts' sha256. `uv sync --frozen` never re-resolves, verifies every hash,
  and builds an isolated environment.
- **The installed distributions describe themselves.** Every installed distribution's `RECORD`
  lists its files with their sha256. FastMCP 4.0.5 is a facade `fastmcp`, plus `fastmcp-slim`
  (257 modules) and `fastmcp-tasks`, which comes through the `tasks` extra.

ADR-0007's revisit trigger fired in slice 1: an analyzer answer changed without `context_id`
changing, because the context did not see the dependency environment.

## Options

1. **The simpler alternative: introspect the project's own `.venv`.** Rejected. Libraries the
   system compiles are not project dependencies, and the project environment carries serving
   and model dependencies (7.8 GB, torch, vLLM) that have nothing to do with the analyzed code.
2. **A bespoke acquisition tool** (ADR-0007 as planned, or the skill's `acquire.py`): download
   wheels and a tarball, write a manifest, build a venv. Rejected: it re-implements resolution,
   hash checking and installation, which uv does, and it is a second lock format to maintain.
3. **One committed uv project per library; acquisition is `uv sync --frozen`; Stage A reads the
   acquired environment.** Chosen.

## Decision

- **A library** is a directory `libraries/<name>/` with three committed files:
  - `pyproject.toml`: a virtual uv project with exactly one pinned requirement, an exact
    `requires-python`, and `[tool.lctx] release`, the first-party distributions whose code is
    compiled. Everything else installed is dependency context, analyzed only as far as imports
    reach. `[tool.lctx.source]` names the upstream tag for docs, examples and tests; it is fetched
    when the docs family lands.
  - `.python-version`: the exact interpreter.
  - `uv.lock`: the acquisition lock.
- **The pilot** is `libraries/fastmcp`:
  - `fastmcp[anthropic,openai,gemini,azure,apps,code-mode,tasks]==4.0.5`, the same install line
    as the `fastmcp` skill;
  - release `fastmcp`, `fastmcp-slim` and `fastmcp-tasks`;
  - Python 3.14.7.
- **Acquisition** is `lctx acquire <name>`:
  - It runs `uv sync --project libraries/<name> --frozen --no-install-project --no-config
    --python <.python-version> --link-mode copy` into `build/envs/<name>` (gitignored,
    rebuildable).
  - Every other `UV_*` variable and `VIRTUAL_ENV` is removed from uv's environment.
  - `--no-config` ignores user and system uv configuration. `--python` enforces the pin.
    `--link-mode copy` keeps the environment's files from sharing inodes with the uv cache and
    other environments.
  - `lctx acquire <name> --reinstall` is the remedy when Stage A finds a changed file.
  - `lctx library init` locks without `--no-config`, because the library's own `[tool.uv]` applies
    there, and the lock is reviewed anyway.
- **One equivalence: the analyzer-readable bytes.** The analyzer-readable files are `.py`,
  `.pyi` and `py.typed`, the files Pyrefly's module finder reads; `.pth` files are never read.
  Stage A (`cpg_extract::library`) reads the definition, the lock, `pyvenv.cfg` and every
  `*.dist-info`, with no network and no interpreter. Its checks:
  - every installed distribution must be the version the lock names;
  - the interpreter must be `.python-version`'s;
  - **every** distribution's analyzer-readable `RECORD` entries must match their sha256;
  - a release distribution the lock records without artifact hashes (a git or local source) is
    refused;
  - a declared `[tool.lctx.source]` must pin a 40-hex `commit`, and its `tag` must name the
    locked version.

  The release's modules are exactly the `.py`/`.pyi` entries of those `RECORD`s; nothing walks a
  directory.
- **Identities.** ADR-0007's are carried forward: BLAKE3 `lctx-id/v1`, content-derived `node_id`
  and `fact_id`, a random per-attempt `snapshot_id`, snapshot-qualified keys, and `run_id` over
  release, context, producer and families. The changes:
  - **`release_id`** hashes the release distributions' names and versions and the sorted
    (path, sha256) of their verified analyzer-readable entries: what is analyzed, not the lock
    entry. A re-listed artifact or a dependency-only upgrade leaves it unchanged. A source tree
    (fixtures, local checkouts) still hashes its label.
  - **`context_id`** adds:
    - the **environment digest**: the installed distributions' dist-info names and every
      analyzer-readable file's site-relative path and content. `RECORD` lines outside
      site-packages (console scripts, which carry the environment's absolute path) never enter
      it, so a moved environment keeps its identity;
    - the **lock digest**.
  - **`content_digest`** takes the run ids, which carry both.
- **Provenance.**
  - `releases`: library, requirement, lock digest, the release distributions (`name==version`),
    the installer from `pyvenv.cfg`, or the tree's label.
  - `distributions`, keyed by `context_id`: the environment belongs to the context. Each row has
    a version, artifact hashes and a `RECORD` digest, with a reference rule to `contexts`.
  - `releases`, `distributions` and `source_files` are written **once per attempt, by the
    extractor run**, which carries Stage A's output. Later producers reference `release_id` and
    never append.
- **One CLI, `lctx`,** is the production path:
  - `library init <name> --requirement REQ` writes the definition and locks it. It then acquires,
    and proposes `release` as the requested distribution plus those sharing its source
    repository, for review;
  - `acquire <name>`;
  - `compile <name> --store DIR`: acquire, Stage A, extract, derive, validate, publish.

  Upgrading is: edit the pin, `uv lock --upgrade-package`, compile.
- **Gold and analysis stay on one FastMCP.** `scripts/check_gold.py` (`just gold`, run by `just test-all`, separate from `just deps`) fails when
  the skill's install line or its resolved release versions differ from `libraries/fastmcp`.

## Consequences

- **Adding a library is data, not code.** On 2026-09-22 (observed, not a repo test), `attrs`
  25.3.0 was defined, locked, acquired and published with every rule passing in 0.8 s, through
  `lctx library init` and `lctx compile`. The same `library init` for FastMCP proposed exactly the
  three first-party distributions, and its lock matched the committed one. The proposal rule is
  **Tested** by `propose.rs`'s unit tests.
- **The pilot runs end to end.**
  - **Measured** (2026-09-22, release build, warm environment and uv cache, acquisition a no-op,
    this Linux host): FastMCP 4.0.5 (275 modules, 103 distributions) extracts in 7.1 s and
    publishes in 7.9 s wall time, at 1.61 GB peak RSS.
  - Every validation rule passes.
  - Two runs give the same `release_id` and `content_digest`.
- **Oracles** (**Tested**; carried from ADR-0007 where noted):
  - in `crates/cpg-extract/tests/library.rs`:
    - a changed analyzer-readable byte is refused, in the release or a dependency;
    - a version drift and an interpreter drift are refused;
    - a hash-less (git) release is refused;
    - `release_id` is the release content (a re-listed artifact leaves it, changed content moves
      it);
    - the environment digest is location-independent and sees an unowned stub inside a package, a
      loose file, and a lock-only change. This is ADR-0007's "changing any single context input
      changes `run_id`";
    - the docs source must be pinned and name the locked version;
    - the release compiles alone, and its imports resolve into the dependencies.
  - in `crates/lctx/tests/acquire.rs`: uv receives `--frozen --no-config --python <pin>
    --link-mode copy`, and no `UV_*`/`VIRTUAL_ENV`.
  - in `crates/lctx/src/propose.rs`: the FastMCP trio is grouped; a shared sponsor link is not a
    shared source; a distribution without a repository is proposed alone.
  - the proptests and known-answer vectors (ADR-0007): the same inputs give the same ids.
- **What costs more.**
  - The first acquisition needs the network.
  - Every library has an environment on disk (FastMCP: 127 MB).
  - The skill must be re-pinned with the library, and the gold guard makes that visible.
- **Review (standard, 2026-09-22).** It found the first recipes wrong.
  - The environment digest hashed whole `RECORD`s, whose console-script lines carry the
    environment's path. It also ignored files inside owned packages.
  - `release_id` followed the lock entry.
  - Acquisition was steerable by user uv configuration.

  All three were corrected as above before acceptance. On the pilot, two environment paths now
  give the same `release_id` and `content_digest`, and every installed file has a link count
  of 1.
- **Superseded.** ADR-0007's acquisition clause (manifest, `ACQUISITION.json`, the skill's
  pattern, the 4.0.3 three-distribution release) is superseded. Everything else it decided is
  carried forward here.

## Amendments

- 2026-09-23: CPG slice C5 (ADR-0014; operator decision "CPG first"). A library's upstream tree
  (`[tool.lctx.source]`) is compiled as a **corpus run** beside the library run, in the same
  attempt. `releases`, `distributions` and `source_files` are written **once per extractor run**:
  an attempt holds several runs only over distinct releases (the library, its corpus), because the
  family tables' keys carry no run. The corpus run analyzes in the library's environment, so it
  shares the library's context, and `distributions` is written once. `lctx acquire` also fetches
  the tree hermetically at its pinned commit into `build/sources/<name>/<commit>`, checked by
  `rev-parse` every time. The corpus release's id hashes `repository@commit` and every selected
  file's path and content. This replaces "once per attempt, by the extractor run" in the
  Provenance clause above.
- 2026-09-23: CPG slice C5b corrects the C5 amendment above. The corpus run does **not** share the
  library's context: to resolve a module both runs import to one file, its search path puts the
  tree ahead of the environment's site-packages (the library run's search path), so its context
  id differs. It runs in the same environment, and its context lists the same `distributions`.
  What the two runs both assert (a dependency module or definition, a type term) is one node,
  written once from its first fact.
- 2026-09-23: H1 C2 (the library-leverage review). The corpus selection reads the fetched tree in one
  walkdir pass that follows no link, and matches globs with globset (`*`, `?`, `**`, `[…]`,
  `{a,b}`, `/` literal). A symlink a glob selects is refused, naming it, and so is a directory link
  that could hold a selection unless an exclude covers it; a source tree refuses any link. The hand
  matcher it replaces followed links (reading outside the tree, looping), read `?`, `[…]` and
  `{…}` as literals and backtracked exponentially.
- 2026-09-23: pointer. The C2 selection semantics above are recorded as their own decision,
  ADR-0018 (the H1 review, F4), with F5's stricter exclude coverage and the `examples_exclude`
  and `tests_exclude` keys.
