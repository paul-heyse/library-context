---
id: ADR-0013
title: Libraries are pinned uv projects; Stage A reads the acquired environment; the pilot is FastMCP 4.0.5
status: proposed
date: 2026-09-22
supersedes: [ADR-0007]
superseded-by: null
design: [§1.2, §1.4, §3.2, §3.4.1, §4.0, §4.1, §12]
evidence: Tested
revisit: A library cannot be expressed as one uv requirement with a first-party distribution list (e.g. it needs a non-PyPI build or a platform uv cannot lock), `uv sync --frozen` stops verifying artifact hashes, or a release's modules are not all listed in its distributions' RECORDs.
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
  - It runs `uv sync --project libraries/<name> --frozen --no-install-project` into
    `build/envs/<name>` (gitignored, rebuildable).
  - Every other `UV_*` variable and `VIRTUAL_ENV` is removed from uv's environment.
- **Stage A** (`cpg_extract::library`) reads the definition, the lock, `pyvenv.cfg` and every
  `*.dist-info`, with no network and no interpreter. Its checks:
  - every installed distribution must be the version the lock names;
  - every file of a release distribution must match its `RECORD` sha256.

  The release's modules are exactly the `.py`/`.pyi` entries of those `RECORD`s; nothing walks a
  directory.
- **Identities.** ADR-0007's are carried forward: BLAKE3 `lctx-id/v1`, content-derived `node_id`
  and `fact_id`, a random per-attempt `snapshot_id`, snapshot-qualified keys, and `run_id` over
  release, context, producer and families. The changes:
  - **`release_id`** hashes the release distributions' names and versions and the sorted sha256s
    of every artifact the lock records for them. A source tree (fixtures, local checkouts) still
    hashes its label.
  - **`context_id`** adds:
    - the **environment digest**: each installed distribution's name, version and `RECORD`
      digest, plus the content of every top-level entry no `RECORD` owns;
    - the **lock digest**.
  - **`content_digest`** takes the run ids, which carry both.
- **Provenance.**
  - `releases`: library, requirement, lock digest, or the tree's label.
  - `distributions`: every installed distribution's version, artifact hashes, `RECORD` digest, and
    whether it is in the release.

  Both are registries the extractor run writes.
- **One CLI, `lctx`,** is the production path:
  - `library init <name> --requirement REQ` writes the definition and locks it. It then acquires,
    and proposes `release` as the requested distribution plus those sharing its source
    repository, for review;
  - `acquire <name>`;
  - `compile <name> --store DIR`: acquire, Stage A, extract, derive, validate, publish.

  Upgrading is: edit the pin, `uv lock --upgrade-package`, compile.
- **Gold and analysis stay on one FastMCP.** `scripts/check_gold.py`, in `just deps`, fails when
  the skill's install line or its resolved release versions differ from `libraries/fastmcp`.

## Consequences

- **Adding a library is data, not code.** `attrs` 25.3.0 was defined, locked, acquired and
  published with every rule passing, in 0.8 s, through `lctx library init` and `lctx compile`
  (2026-09-22). The same `library init` for FastMCP proposes exactly the three first-party
  distributions, and its lock matches the committed one.
- **The pilot runs end to end.**
  - **Measured** (2026-09-22, `just pilot`, release build): FastMCP 4.0.5 (275 modules, 103
    distributions) extracts in 7.1 s and publishes in 8.1 s wall time, at 1.66 GB peak RSS.
  - Every validation rule passes.
  - Two runs give the same `release_id` and `content_digest`.
- **Oracles** (`crates/cpg-extract/tests/library.rs`, **Tested**):
  - a file that differs from its `RECORD` is refused;
  - an environment not synced to the lock is refused;
  - `release_id` follows the release distributions' locked artifacts only;
  - the environment digest follows `RECORD`s and loose files, but not bytes a `RECORD` owns;
  - the release compiles alone, and its imports resolve into the dependencies.
- **What costs more.**
  - The first acquisition needs the network.
  - Every library has an environment on disk (FastMCP: 127 MB).
  - The skill must be re-pinned with the library, and the gold guard makes that visible.
- **What we give up.** Tampering with a dependency's installed bytes is not detected unless its
  `RECORD` changes. The release's own bytes are always verified.
- **Superseded.** ADR-0007's acquisition clause (manifest, `ACQUISITION.json`, the skill's
  pattern, the 4.0.3 three-distribution release) is superseded. Everything else it decided is
  carried forward here.
